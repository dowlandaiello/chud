#[macro_use]
extern crate leptos;

use async_channel::Receiver;
use chud::{net::client::Client, rpc::cmd::CmdResp};
use futures::future::FutureExt;
use leptos::{component, For, IntoView, ReadSignal, Scope, SignalUpdate, WriteSignal};
use std::{future::Future, sync::Arc};

const BOOTSTRAP_NODES: [&'static str; 1] = ["/ip4/127.0.0.1/tcp/6224/wss"];
const ENTER_KEY_KEYCODE: u32 = 13;

#[component]
fn App(cx: Scope) -> impl IntoView {
	// Events on the UI
	let events_signal = Arc::new(leptos::create_signal(cx, Vec::new()));
	let sig = events_signal.clone();
	let (events, set_events) = *events_signal;

	// Input for commands
	let (cmd, set_cmd) = leptos::create_signal(cx, "".to_owned());

	let (tx, rx) = async_channel::unbounded();
	let (tx_resp, rx_resp) = async_channel::unbounded();

	wasm_bindgen_futures::spawn_local(async move {
		let (_, sig) = *sig;

		// Start the client
		wasm_bindgen_futures::spawn_local(async {
			// Start the client with some known bootstrap nodes
			let client = Client::load_from_disk(0)
				.await
				.expect("to be able to load the blockchain from the disk");

			let _ = client
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
					None,
				)
				.await;
		});

		async fn poll(set_events: WriteSignal<Vec<(CmdResp, usize)>>, rx_resp: Receiver<CmdResp>) {
			loop {
				let val = rx_resp.recv().await;
				set_events
					.update(|events| events.push((val.expect("to be an ok value"), events.len())));
			}
		}

		poll(sig, rx_resp).await;
	});

	view! { cx, <For each=events key=|x| x.1 view=move |cx, event: (CmdResp, usize)| {
		view! {cx,
			<p>{format!("{}", serde_json::to_string_pretty(&event.0).expect("request to succeed"))}</p>
		}
	} />
			<input type="text" on:input=move |ev| {
					set_cmd(leptos::event_target_value(&ev));
				}
				on:keydown=move |ev| {
					let tx = tx.clone();

					if ev.key_code() == ENTER_KEY_KEYCODE { match serde_json::from_str(cmd().as_str()) {
						Ok(cmd) => wasm_bindgen_futures::spawn_local(async move { tx.send(cmd).await.expect("channel send to succeed")}),
						Err(e) => set_events.update(|events| events.push((CmdResp::Error { error: e.to_string(), req_id: 0 }, events.len()))),
					}}
				}
			/>
	}
}

fn main() {
	wasm_logger::init(wasm_logger::Config::default());

	leptos::mount_to_body(|cx| view! {cx,  <App />});
}
