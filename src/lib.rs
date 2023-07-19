#[macro_use]
extern crate log;

pub mod captcha;
pub mod crypto;
pub mod net;
pub mod rpc;
pub mod sys;
pub mod util;

#[cfg(target_arch = "wasm32")]
use async_channel::{Receiver, Sender};

#[cfg(target_arch = "wasm32")]
use crypto::hash::Hash;
#[cfg(target_arch = "wasm32")]
use js_sys::Function;
#[cfg(target_arch = "wasm32")]
use net::client::Error;
#[cfg(target_arch = "wasm32")]
use rpc::cmd::{Cmd, CmdResp, LoadMsgReq, SubmitMsgReq};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;

#[cfg(target_arch = "wasm32")]
use futures::future::FutureExt;

#[cfg(target_arch = "wasm32")]
lazy_static::lazy_static! {
	static ref CMD_RX_TX: (Sender<Cmd>, Receiver<Cmd>) = async_channel::unbounded();
}

#[cfg(target_arch = "wasm32")]
lazy_static::lazy_static! {
	static ref RESP_RX_TX: (Sender<CmdResp>, Receiver<CmdResp>) = async_channel::unbounded();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start(chain_id: usize, bootstrap_nodes: Vec<js_sys::JsString>) {
	wasm_logger::init(wasm_logger::Config::default());

	// Convert JS strings to str's
	let nodes_owned = bootstrap_nodes
		.into_iter()
		.filter_map(|js_str| js_str.as_string())
		.collect::<Vec<String>>();

	let mut client = net::client::Client::new(chain_id);
	wasm_bindgen_futures::spawn_local(
		client
			.start(
				CMD_RX_TX.1.clone(),
				RESP_RX_TX.0.clone(),
				nodes_owned,
				None,
				Vec::new(),
				None,
			)
			.map(|_| ()),
	);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn submit_message(msg_data: wasm_bindgen::JsValue) -> Result<String, String> {
	let msg = serde_wasm_bindgen::from_value(msg_data)
		.map_err(|e| Error::SerdeWasmError(e))
		.map_err(|e| e.to_string())?;
	let req_id = instant::now() as usize;
	CMD_RX_TX
		.0
		.send(Cmd::SubmitMsg { req: msg, req_id })
		.await
		.map_err(|e| e.to_string())?;

	loop {
		match RESP_RX_TX.1.recv().await.map_err(|e| e.to_string())? {
			CmdResp::MsgSubmitted {
				hash: h,
				req_id: resp_id,
			} => {
				if resp_id == req_id {
					return Ok(hex::encode(h));
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if req_id == resp_id {
					return Err(format!("Failed to submit the message: {}", error));
				}
			}
			_ => continue,
		}
	}
}

/// Gets a JSON encoding of the message with the given hash.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn load_message(hash_str: &str) -> Result<JsValue, String> {
	let hash_bytes = hex::decode(hash_str).map_err(|e| e.to_string())?;
	let mut hash_bytes_arr: [u8; 32] = [0; 32];

	for i in 0..32 {
		hash_bytes_arr[i] = hash_bytes[i];
	}

	let hash: Hash = hash_bytes_arr.into();
	let req_id = instant::now() as usize;
	CMD_RX_TX
		.0
		.send(Cmd::LoadMsg {
			req: LoadMsgReq { hash },
			req_id: req_id as usize,
		})
		.await
		.map_err(|e| e.to_string())?;

	loop {
		match RESP_RX_TX.1.recv().await.map_err(|e| e.to_string())? {
			CmdResp::MsgLoaded {
				msg,
				req_id: resp_id,
			} => {
				if resp_id == req_id as usize {
					return Ok(serde_wasm_bindgen::to_value(&msg).map_err(|e| e.to_string())?);
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if resp_id == req_id as usize {
					return Err(format!("Error occurred while loading message: {}", error));
				}
			}
			_ => continue,
		}
	}
}

/// Gets a hex encoding of the hash of the HEAD message for the chain.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn get_head() -> Result<String, String> {
	let req_id = instant::now() as usize;
	CMD_RX_TX
		.0
		.send(Cmd::GetHead {
			req_id: req_id as usize,
		})
		.await
		.map_err(|e| e.to_string())?;

	loop {
		match RESP_RX_TX.1.recv().await.map_err(|e| e.to_string())? {
			CmdResp::HeadLoaded {
				hash,
				req_id: resp_id,
			} => {
				if resp_id == req_id as usize {
					return Ok(hex::encode(hash));
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if resp_id == req_id as usize {
					return Err(format!("Error occurred while loading message: {}", error));
				}
			}
			_ => continue,
		}
	}
}

/// Registers a callback to be executed every time a new message is received.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn on_message(callback: Function) {
	wasm_bindgen_futures::spawn_local(async move {
		loop {
			match RESP_RX_TX.1.recv().await.map_err(|e| e.to_string()) {
				Ok(CmdResp::MsgReceived { msg }) => {
					let json = match serde_wasm_bindgen::to_value(&msg).map_err(|e| e.to_string()) {
						Ok(v) => v,
						Err(e) => {
							error!("Error occurred while serializing message: {}", e);
							continue;
						}
					};

					let this = JsValue::null();
					callback.call1(&this, &json);
				}
				Err(e) => {
					error!("Error occurred while listening to messages: {}", e);
				}
				_ => {}
			}
		}
	});
}
