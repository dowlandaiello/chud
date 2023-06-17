use super::super::crypto::hash::Hash;
use serde::{Deserialize, Serialize};

/// A request for some information from a peer.
#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
	/// Requests the peer for the hash of the longest chain
	LongestChain,
}

/// A response for some information from a peer.
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
	LongestChain { height: usize, hash: Hash },
}
