#[macro_use]
extern crate leptos;

use chud::net::client::Client;

const BOOTSTRAP_NODES: [&'static str; 1] = ["/ip4/127.0.0.1/tcp/6224/ws"];

fn main() {
	wasm_logger::init(wasm_logger::Config::default());

	// Start the client
	let (tx, rx) = async_channel::unbounded();
	let (tx_resp, rx_resp) = async_channel::unbounded();
	wasm_bindgen_futures::spawn_local(async {
		// Start the client with some known bootstrap nodes
		let client = Client::load_from_disk(0)
			.await
			.expect("to be able to load the blockchain from the disk");

		client
			.start(
				rx,
				tx_resp,
				BOOTSTRAP_NODES
					.to_vec()
					.into_iter()
					.map(|x| x.to_owned())
					.collect::<Vec<String>>(),
				None,
				Vec::new(),
			)
			.await;
	});

	leptos::mount_to_body(|cx| view! { cx, <p>"hi"</p> })
}
