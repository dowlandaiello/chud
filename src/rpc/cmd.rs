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
	data: Vec<u8>,
	prev: Option<Hash>,
	captcha_ans: String,
	height: usize,
}

/// RPC outputs to the CHUD CLI.
pub enum CmdResp {
	MsgSubmitted(Hash),
}
