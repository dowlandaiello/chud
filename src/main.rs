use chud::net::client::Client;

#[cfg(not(target_arch = "wasm32"))]
use actix_web::{web::Data, App, HttpServer};
#[cfg(not(target_arch = "wasm32"))]
use chud::rpc::{get_head, health_check, load_msg, submit_msg, terminate};
#[cfg(not(target_arch = "wasm32"))]
use clap::{arg, command, Parser};
#[cfg(not(target_arch = "wasm32"))]
use futures::TryFutureExt;
use libp2p::Multiaddr;
#[cfg(not(target_arch = "wasm32"))]
use std::error::Error;

/// Arguments to chudd:
/// --chain-id: The unique segregator for the blockchain. Should be the same
/// across clients on the same network.
/// --bootstrap-peers: A list of multiaddrs representing the peers to bootstrap
/// the chain from.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	#[arg(short, long, default_value_t = 0)]
	chain_id: usize,

	#[arg(short, long)]
	bootstrap_peers: Vec<String>,

	#[arg(short, long, default_value_t = 6224)]
	port: u16,

	#[arg(short, long, default_value_t = 8080)]
	rpc_port: u16,

	#[arg(short, long)]
	external_addrs: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let args = Args::parse();

	// Start the client
	let (tx, rx) = async_channel::unbounded();
	let (tx_resp, rx_resp) = async_channel::unbounded();

	// Start the RPC server
	let server_fut = HttpServer::new(move || {
		App::new()
			.app_data(Data::new(tx.clone()))
			.app_data(Data::new(rx_resp.clone()))
			.service(get_head)
			.service(submit_msg)
			.service(load_msg)
			.service(terminate)
			.service(health_check)
	})
	.bind(("0.0.0.0", args.rpc_port))?
	.run();

	let client = Client::load_from_disk(args.chain_id)
		.await
		.expect("Failed to load client");
	let client_fut = client.start(
		rx,
		tx_resp,
		args.bootstrap_peers,
		Some(args.port),
		args.external_addrs
			.into_iter()
			.filter_map(|s| s.parse().ok())
			.collect::<Vec<Multiaddr>>(),
	);

	futures::try_join!(
		server_fut.map_err(|e| e.into()),
		client_fut.map_err(|e| e.into())
	)
	.map(|_| ())
}

#[cfg(target_arch = "wasm32")]
fn main() {}
