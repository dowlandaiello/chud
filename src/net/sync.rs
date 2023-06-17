use super::{
	super::{crypto::hash::Hash, sys::rt::Rt},
	DHT_QUORUM,
};
use libp2p::kad::{
	record::store::Error as KadError, store::MemoryStore, Kademlia, Record, RecordKey,
};
use serde_json::Error as SerdeError;
use std::{
	error::Error as StdError,
	fmt::{Display, Error as FmtError, Formatter},
};

/// Any error that may occur while synchronizing the blockchain.
#[derive(Debug)]
pub enum Error {
	MissingMessage,
	SerializationError(SerdeError),
	KadError(KadError),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
		match self {
			Error::MissingMessage => write!(f, "the runtime is missing the requested message"),
			Error::SerializationError(e) => write!(f, "serialization error while syncing: {}", e),
			Error::KadError(e) => write!(f, "DHT error: {}", e),
		}
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Error::MissingMessage => None,
			Error::SerializationError(e) => Some(e),
			Error::KadError(e) => Some(e),
		}
	}
}

impl From<KadError> for Error {
	fn from(e: KadError) -> Self {
		Self::KadError(e)
	}
}

/// Commits all transactions in the client's blockchain to the DHT.
pub fn upload_chain(rt: &Rt, kad: &mut Kademlia<MemoryStore>) -> Result<(), Error> {
	// Only continue synchronizing if some chain data exists
	let longest_chain = if let Some(h) = rt.longest_chain() {
		h
	} else {
		return Ok(());
	};

	info!("committing chain {} to DHT", hex::encode(longest_chain));

	// Write the message with hash has to the blockchain, as well as its predecessor
	fn upload_chain_from(
		rt: &Rt,
		curr: &Hash,
		kad: &mut Kademlia<MemoryStore>,
	) -> Result<(), Error> {
		let msg = rt.get_message(&curr).ok_or(Error::MissingMessage)?;
		let msg_bytes = serde_json::to_vec(&msg).map_err(|e| Error::SerializationError(e))?;

		// Write the transaction under its hash with its JSON serialization to the DHT
		debug!(
			"writing message {} to KAD DHT",
			hex::encode(msg.hash().as_ref())
		);
		kad.put_record(
			Record::new(RecordKey::new(&msg.hash().as_ref()), msg_bytes),
			DHT_QUORUM,
		)?;

		if let Some(prev) = msg.data().prev() {
			upload_chain_from(rt, prev, kad)
		} else {
			Ok(())
		}
	}

	upload_chain_from(rt, longest_chain, kad)
}
