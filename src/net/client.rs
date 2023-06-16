use super::{
	super::{
		rpc::cmd::{Cmd, CmdResp},
		sys::rt::Rt,
	},
	behavior::Behavior,
};
use async_channel::{Receiver, Sender};
use futures::select;
use indexed_db_futures::{
	idb_transaction::IdbTransaction, prelude::IdbTransactionMode, request::IdbOpenDbRequestLike,
	IdbDatabase, IdbQuerySource, IdbVersionChangeEvent,
};
use libp2p::{
	core::{upgrade::Version, ConnectedPoint},
	floodsub::Floodsub,
	futures::StreamExt,
	identify::{Behaviour, Config},
	identity,
	kad::{record::store::MemoryStore, Kademlia, NoKnownPeers},
	multiaddr::Error as MultiaddrError,
	noise::{Config as NoiseConfig, Error as NoiseError},
	swarm::{DialError, SwarmBuilder, SwarmEvent},
	yamux::Config as YamuxConfig,
	Multiaddr, PeerId, Transport,
};
use libp2p_websys_transport::WebsocketTransport;
use serde::{Deserialize, Serialize};
use std::{
	error::Error as StdError,
	fmt::{Display, Error as FmtError, Formatter},
};
use wasm_bindgen::JsValue;
use web_sys::DomException;

/// The name of the indexed db in which chud data is stored.
pub const DB_NAME: &'static str = "chud_db";

/// The object store in which runtime data is stored.
pub const RUNTIME_STORE: &'static str = "runtime";

/// The key under the runtime store under which the state is stored.
pub const STATE_KEY: &'static str = "state";

/// The name to be broadcasted by P2P peers to identify each other.
pub const NET_PROTOCOL_PREFIX: &'static str = "chud_";

/// An error that could be encountered by the client.
#[derive(Debug)]
pub enum Error {
	NoiseError(NoiseError),
	MultiaddrError(MultiaddrError),
	NoKnownPeers,
	DialError(DialError),
}

impl From<NoiseError> for Error {
	fn from(e: NoiseError) -> Self {
		Self::NoiseError(e)
	}
}

impl From<MultiaddrError> for Error {
	fn from(e: MultiaddrError) -> Self {
		Self::MultiaddrError(e)
	}
}

impl From<NoKnownPeers> for Error {
	fn from(_: NoKnownPeers) -> Self {
		Self::NoKnownPeers
	}
}

impl From<DialError> for Error {
	fn from(e: DialError) -> Self {
		Self::DialError(e)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::NoiseError(e) => Some(e),
			Self::MultiaddrError(e) => Some(e),
			Self::NoKnownPeers => Some(&NoKnownPeers()),
			Self::DialError(e) => Some(e),
		}
	}
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
		match self.source() {
			Some(e) => write!(f, "Client encountered an error: {}", e),
			None => write!(f, "Client encountered an error"),
		}
	}
}

/// An interface with the CHUD network.
#[derive(Serialize, Deserialize)]
pub struct Client {
	pub runtime: Rt,
	chain_id: usize,
	bootstrapped: bool,
}

impl Client {
	/// Creates a new runtime with the given chain ID.
	pub fn new(chain_id: usize) -> Self {
		Self {
			chain_id,
			runtime: Rt::default(),
			bootstrapped: false,
		}
	}

	/// Loads the saved blockchain data from indexeddb.
	pub async fn load_from_disk(chain_id: usize) -> Result<Self, DomException> {
		let mut client = Client::new(chain_id);

		// Create the object store if it doesn't exist
		let mut db_req = IdbDatabase::open(DB_NAME)?;
		db_req.set_on_upgrade_needed(Some(|e: &IdbVersionChangeEvent| -> Result<(), JsValue> {
			if let None = e.db().object_store_names().find(|n| n == RUNTIME_STORE) {
				e.db().create_object_store(RUNTIME_STORE)?;
			}

			Ok(())
		}));

		// Open the database
		let db: IdbDatabase = db_req.into_future().await?;

		// Read the state from the database
		let tx: IdbTransaction =
			db.transaction_on_one_with_mode(RUNTIME_STORE, IdbTransactionMode::Readonly)?;
		let store = tx.object_store(RUNTIME_STORE)?;

		if let Some(Ok(record)) = store.get_owned(STATE_KEY)?.await.and_then(|val| {
			Ok(val.map(|val| {
				serde_wasm_bindgen::from_value(val)
					.map_err(|e| DomException::from(JsValue::from_str(format!("{}", e).as_str())))
			}))
		})? {
			client = record;
		}

		Ok(client)
	}

	/// Writes the blockchain to indexeddb.
	pub async fn write_to_disk(&self) -> Result<(), DomException> {
		// Create the object store if it doesn't exist
		let mut db_req = IdbDatabase::open(DB_NAME)?;
		db_req.set_on_upgrade_needed(Some(|e: &IdbVersionChangeEvent| -> Result<(), JsValue> {
			if let None = e.db().object_store_names().find(|n| n == RUNTIME_STORE) {
				e.db().create_object_store(RUNTIME_STORE)?;
			}

			Ok(())
		}));

		// Open the database
		let db: IdbDatabase = db_req.into_future().await?;

		// Write the state to the database
		let tx: IdbTransaction =
			db.transaction_on_one_with_mode(RUNTIME_STORE, IdbTransactionMode::Readwrite)?;
		let store = tx.object_store(RUNTIME_STORE)?;

		store.put_key_val_owned(
			STATE_KEY,
			&serde_wasm_bindgen::to_value(self)
				.map_err(|e| DomException::from(JsValue::from_str(format!("{}", e).as_str())))?,
		)?;

		Ok(())
	}

	/// Synchronizes and keep sthe client in sync with the network. Accepts
	/// commands on a receiving channel for operations to perform.
	pub async fn start(
		&mut self,
		mut cmd_rx: Receiver<Cmd>,
		resp_tx: Sender<CmdResp>,
		bootstrap_peers: &[&str],
	) -> Result<(), Error> {
		// Use WebSockets as a transport.
		// TODO: Use webrtc in the future for p2p in browsers
		let local_key = identity::Keypair::generate_ed25519();
		let local_peer_id = PeerId::from(local_key.public());
		let transport = WebsocketTransport::default()
			.upgrade(Version::V1Lazy)
			.authenticate(NoiseConfig::new(&local_key)?)
			.multiplex(YamuxConfig::default())
			.boxed();

		// Create a swarm with the desired behavior
		let mut swarm = {
			let store = MemoryStore::new(local_peer_id);
			let kad = Kademlia::new(local_peer_id, store);
			let floodsub = Floodsub::new(local_peer_id);
			let identify = Behaviour::new(Config::new(
				format!("{}{}", NET_PROTOCOL_PREFIX, self.chain_id),
				local_key.public(),
			));

			SwarmBuilder::with_wasm_executor(
				transport,
				Behavior::new(kad, floodsub, identify),
				local_peer_id,
			)
			.build()
		};

		// Dial all bootstrap peers
		for multiaddr in bootstrap_peers {
			swarm
				.dial(
					multiaddr
						.parse::<Multiaddr>()
						.map_err(<MultiaddrError as Into<Error>>::into)?,
				)
				.map_err(<DialError as Into<Error>>::into)?;
		}

		loop {
			select! {
				event = swarm.select_next_some() => match event {
					SwarmEvent::Behaviour(event) => {
						println!("{:?}", event);
					},
					SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
						// Register peers in the kademlia DHT and floodsub once they're found
						match endpoint {
							ConnectedPoint::Dialer {
								address, ..
							} => {
								swarm.behaviour_mut().kad_mut().add_address(&peer_id, address.clone());

								// Bootstrap the DHT if we connected to one of the bootstrap addresses
								if !self.bootstrapped && bootstrap_peers.contains(&(address.to_string().as_str())) {
									swarm.behaviour_mut().kad_mut().bootstrap().map_err(<NoKnownPeers as Into<Error>>::into)?;

									self.bootstrapped = true;
								}

							},
							_ => {}
						}

						swarm.behaviour_mut().floodsub_mut().add_node_to_partial_view(peer_id);
					},
					SwarmEvent::ConnectionClosed { peer_id, ..} => {
						// Remove disconnected peers
						swarm.behaviour_mut().kad_mut().remove_peer(&peer_id);
						swarm.behaviour_mut().floodsub_mut().remove_node_from_partial_view(&peer_id);
					}
					_ => {}
				},
				cmd = cmd_rx.select_next_some() => match cmd {
					Cmd::Terminate => break Ok(()),
					_ => println!("{:?}", cmd),
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new() {
		let client = Client::new(0);
		assert_eq!(client.chain_id, 0);
	}
}
