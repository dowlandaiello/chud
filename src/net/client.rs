use super::{
	super::{
		rpc::cmd::{Cmd, CmdResp},
		sys::rt::Rt,
	},
	behavior::Behavior,
	sync::{self, Error as SyncError},
	DAEMON_PORT, DB_NAME, NET_PROTOCOL_PREFIX, RR_PROTOCOL_PREFIX, RUNTIME_STORE, STATE_KEY,
	SYNCHRONIZATION_INTERVAL,
};
use async_channel::{Receiver, Sender};
use futures::{future::FutureExt, select};
#[cfg(target_arch = "wasm32")]
use indexed_db_futures::{
	idb_transaction::IdbTransaction, prelude::IdbTransactionMode, request::IdbOpenDbRequestLike,
	IdbDatabase, IdbQuerySource, IdbVersionChangeEvent,
};
use libp2p::{
	core::{transport::Transport, upgrade::Version, ConnectedPoint},
	floodsub::Floodsub,
	futures::StreamExt,
	identify::{Behaviour, Config},
	identity,
	kad::{record::store::MemoryStore, Kademlia, NoKnownPeers},
	multiaddr::{Error as MultiaddrError, Protocol},
	noise::{Config as NoiseConfig, Error as NoiseError},
	request_response::{cbor::Behaviour as RRBehavior, Config as RRConfig, ProtocolSupport},
	swarm::{DialError, StreamProtocol, Swarm, SwarmBuilder, SwarmEvent},
	yamux::Config as YamuxConfig,
	Multiaddr, PeerId, TransportError,
};
#[cfg(not(target_arch = "wasm32"))]
use libp2p::{
	tcp::{tokio::Transport as TcpTransport, Config as TcpConfig},
	websocket::WsConfig,
};
#[cfg(target_arch = "wasm32")]
use libp2p_websys_transport::WebsocketTransport;
use serde::{Deserialize, Serialize};
use std::{
	cfg,
	error::Error as StdError,
	fmt::{Display, Error as FmtError, Formatter},
	net::Ipv4Addr,
	time::Duration,
};
use tokio::time;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use web_sys::DomException;

#[cfg(not(target_arch = "wasm32"))]
use tokio::{
	fs::File,
	io::{AsyncReadExt, AsyncWriteExt, Error as TokioError, Result as TokioResult},
};

use std::io::{Error as IoError, ErrorKind};

/// An error that could be encountered by the client.
#[derive(Debug)]
pub enum Error {
	NoiseError(NoiseError),
	MultiaddrError(MultiaddrError),
	NoKnownPeers,
	DialError(DialError),
	TransportError(TransportError<IoError>),
	SyncError(SyncError),
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

impl From<TransportError<IoError>> for Error {
	fn from(e: TransportError<IoError>) -> Self {
		Self::TransportError(e)
	}
}

impl From<SyncError> for Error {
	fn from(e: SyncError) -> Self {
		Self::SyncError(e)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::NoiseError(e) => Some(e),
			Self::MultiaddrError(e) => Some(e),
			Self::NoKnownPeers => Some(&NoKnownPeers()),
			Self::DialError(e) => Some(e),
			Self::TransportError(e) => Some(e),
			Self::SyncError(e) => Some(e),
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

	// State variables
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
	#[cfg(target_arch = "wasm32")]
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

	/// Loads the saved blockchain data from a JSON file
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn load_from_disk(chain_id: usize) -> TokioResult<Self> {
		let mut client = Client::new(chain_id);

		// Read the entire database file, and then deserialize it
		if let Ok(mut f) = File::open(DB_NAME).await {
			let mut contents = Vec::new();
			f.read_to_end(&mut contents).await?;

			client = serde_json::from_slice(contents.as_slice())
				.map_err(|e| TokioError::new(ErrorKind::InvalidData, e))?;
		};

		Ok(client)
	}

	/// Writes the blockchain to indexeddb.
	#[cfg(target_arch = "wasm32")]
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

	/// Saves the blockchain to a database file in JSON format.
	#[cfg(not(target_arch = "wasm32"))]
	pub async fn write_to_disk(&self) -> TokioResult<()> {
		// Open the database file and write the serialized blockchain to it
		let mut f = File::create(DB_NAME).await?;

		let ser =
			serde_json::to_vec(self).map_err(|e| TokioError::new(ErrorKind::InvalidData, e))?;
		f.write_all(ser.as_slice()).await?;

		Ok(())
	}

	#[cfg(target_arch = "wasm32")]
	fn build_swarm(&self) -> Result<Swarm<Behavior>, Error> {
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
		{
			let store = MemoryStore::new(local_peer_id);
			let kad = Kademlia::new(local_peer_id, store);
			let floodsub = Floodsub::new(local_peer_id);
			let identify = Behaviour::new(Config::new(
				format!("{}{}", NET_PROTOCOL_PREFIX, self.chain_id),
				local_key.public(),
			));
			let rresponse = RRBehavior::new(
				[(
					StreamProtocol::new(RR_PROTOCOL_PREFIX),
					ProtocolSupport::Full,
				)],
				RRConfig::default(),
			);

			Ok(SwarmBuilder::with_wasm_executor(
				transport,
				Behavior::new(kad, floodsub, identify, rresponse),
				local_peer_id,
			)
			.build())
		}
	}

	#[cfg(not(target_arch = "wasm32"))]
	fn build_swarm(&self) -> Result<Swarm<Behavior>, Error> {
		// Use WebSockets as a transport.
		// TODO: Use webrtc in the future for p2p in browsers
		let local_key = identity::Keypair::generate_ed25519();
		let local_peer_id = PeerId::from(local_key.public());

		let transport = WsConfig::new(TcpTransport::new(TcpConfig::new()))
			.upgrade(Version::V1Lazy)
			.authenticate(NoiseConfig::new(&local_key)?)
			.multiplex(YamuxConfig::default())
			.boxed();

		// Create a swarm with the desired behavior
		{
			let store = MemoryStore::new(local_peer_id);
			let kad = Kademlia::new(local_peer_id, store);
			let floodsub = Floodsub::new(local_peer_id);
			let identify = Behaviour::new(Config::new(
				format!("{}{}", NET_PROTOCOL_PREFIX, self.chain_id),
				local_key.public(),
			));
			let rresponse = RRBehavior::new(
				[(
					StreamProtocol::new(RR_PROTOCOL_PREFIX),
					ProtocolSupport::Full,
				)],
				RRConfig::default(),
			);

			Ok(SwarmBuilder::with_tokio_executor(
				transport,
				Behavior::new(kad, floodsub, identify, rresponse),
				local_peer_id,
			)
			.build())
		}
	}

	/// Synchronizes and keep sthe client in sync with the network. Accepts
	/// commands on a receiving channel for operations to perform.
	pub async fn start(
		mut self,
		mut cmd_rx: Receiver<Cmd>,
		resp_tx: Sender<CmdResp>,
		bootstrap_peers: Vec<String>,
	) -> Result<(), Error> {
		let mut swarm = self.build_swarm()?;

		// Dial all bootstrap peers
		for multiaddr in bootstrap_peers.iter() {
			swarm
				.dial(
					multiaddr
						.parse::<Multiaddr>()
						.map_err(<MultiaddrError as Into<Error>>::into)?,
				)
				.map_err(<DialError as Into<Error>>::into)?;
		}

		if !cfg!(target_arch = "wasm32") {
			// Listen for connections on the given port.
			let address = Multiaddr::from(Ipv4Addr::UNSPECIFIED)
				.with(Protocol::Tcp(DAEMON_PORT))
				.with(Protocol::Ws("/".into()));
			swarm
				.listen_on(address.clone())
				.map_err(<TransportError<IoError> as Into<Error>>::into)?;

			info!("p2p client listening on {}", address);
		}

		// Write all transactions to the DHT and synchronize the chain
		// every n minutes
		let mut sync_fut = time::interval(Duration::from_millis(SYNCHRONIZATION_INTERVAL));

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
								if !self.bootstrapped && bootstrap_peers.contains(&address.to_string()) {
									swarm.behaviour_mut().kad_mut().bootstrap().map_err(<NoKnownPeers as Into<Error>>::into)?;

									self.bootstrapped = true;

									info!("successfully bootstrapped to peer {}", peer_id);
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
				},
				_ = sync_fut.tick().fuse() => {
					sync::upload_chain(&self.runtime, swarm.behaviour_mut().kad_mut())?;
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::super::super::sys::msg::{Message, MessageData};
	use super::*;

	#[test]
	fn test_new() {
		let client = Client::new(0);
		assert_eq!(client.chain_id, 0);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::test]
	async fn test_write_load() -> Result<(), Box<dyn StdError>> {
		let mut client = Client::new(0);

		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg = Message::try_from(data)?;

		// Insert the message
		client.runtime.insert_message(msg);

		// Ensure read and written clients are the same
		client.write_to_disk().await?;
		let client2 = Client::load_from_disk(0).await?;

		assert_eq!(client.runtime, client2.runtime);

		Ok(())
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[tokio::test]
	async fn test_start() -> Result<(), Box<dyn StdError>> {
		let (tx, rx) = async_channel::unbounded();
		let (tx_resp, _) = async_channel::unbounded();
		tx.send(Cmd::Terminate).await?;

		let client = Client::new(0);
		client.start(rx, tx_resp, Vec::new()).await?;

		Ok(())
	}
}
