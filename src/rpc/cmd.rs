use super::super::crypto::hash::Hash;
use serde::{Deserialize, Serialize};

/// RPC inputs to the CHUD CLI.
#[derive(Debug)]
pub enum Cmd {
	SubmitMsg(SubmitMsgReq),
	Terminate,
}

/// A partially applied MessageData construction used by clients to submit
/// messages to the blockchain.
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitMsgReq {
	pub(crate) data: Vec<u8>,
	pub(crate) prev: Option<Hash>,
	pub(crate) captcha_ans: String,
	pub(crate) captcha_src: Hash,
	pub(crate) height: usize,
	pub(crate) timestamp: u128,
}

/// RPC outputs to the CHUD CLI.
pub enum CmdResp {
	MsgSubmitted(Hash),
}
