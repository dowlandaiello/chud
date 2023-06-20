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
use rpc::cmd::{Cmd, CmdResp};

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
			)
			.map(|_| ()),
	);
}
