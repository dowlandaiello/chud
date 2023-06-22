use super::super::{crypto::hash::Hash, sys::msg::Message};
use serde::{Deserialize, Serialize};

/// RPC inputs to the CHUD CLI.
#[derive(Debug)]
pub enum Cmd {
	SubmitMsg { req: SubmitMsgReq, req_id: usize },
	LoadMsg { req: LoadMsgReq, req_id: usize },
	GetHead { req_id: usize },
	Terminate,
}

/// A partially applied MessageData construction used by clients to submit
/// messages to the blockchain.
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitMsgReq {
	pub(crate) data: Vec<u8>,
	pub(crate) prev: Option<Hash>,
	pub(crate) captcha_ans: Option<String>,
	pub(crate) captcha_src: Option<Hash>,
	pub(crate) height: usize,
	pub(crate) timestamp: u128,
}

/// A request to load a message with a particular hash.
#[derive(Serialize, Deserialize, Debug)]
pub struct LoadMsgReq {
	pub(crate) hash: Hash,
}

/// RPC outputs to the CHUD CLI.
#[derive(Debug, PartialEq)]
pub enum CmdResp {
	MsgSubmitted { hash: Hash, req_id: usize },
	MsgLoaded { msg: Message, req_id: usize },
	HeadLoaded { hash: Hash, req_id: usize },
	Error { error: String, req_id: usize },
}
