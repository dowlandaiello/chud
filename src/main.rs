use chud::net::client::Client;

#[cfg(not(target_arch = "wasm32"))]
use clap::{arg, command, Parser};

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
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
	env_logger::init();

	let args = Args::parse();

	// Start the client
	let (_, rx) = async_channel::unbounded();
	let (tx_resp, _) = async_channel::unbounded();

	let client = Client::new(args.chain_id);
	client
		.start(rx, tx_resp, args.bootstrap_peers)
		.await
		.unwrap();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
