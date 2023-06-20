use chud::net::client::Client;

#[cfg(not(target_arch = "wasm32"))]
use clap::{arg, command, Parser};
use libp2p::Multiaddr;

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

	#[arg(short, long)]
	external_addrs: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
	env_logger::init();

	let args = Args::parse();

	// Start the client
	let (_, rx) = async_channel::unbounded();
	let (tx_resp, _) = async_channel::unbounded();

	let client = Client::load_from_disk(args.chain_id)
		.await
		.expect("Failed to load client");
	client
		.start(
			rx,
			tx_resp,
			args.bootstrap_peers,
			Some(args.port),
			args.external_addrs
				.into_iter()
				.filter_map(|s| s.parse().ok())
				.collect::<Vec<Multiaddr>>(),
		)
		.await
		.unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
