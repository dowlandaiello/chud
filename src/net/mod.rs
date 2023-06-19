pub mod behavior;
pub mod client;
pub mod msg;
pub mod req;
pub mod sync;

use libp2p::kad::Quorum;

/// The percentage of the known network that must consent to a DHT change.
pub const DHT_QUORUM: Quorum = Quorum::One;

/// The percentage of the known network that should be sampled for synchronization
/// requests.
pub const SAMPLING_SIZE: f32 = 0.5;

/// The name of the indexed db in which chud data is stored.
pub const DB_NAME: &'static str = "chud_db";

/// The object store in which runtime data is stored.
pub const RUNTIME_STORE: &'static str = "runtime";

/// The key under the runtime store under which the state is stored.
pub const STATE_KEY: &'static str = "state";

/// The name to be broadcasted by P2P peers to identify each other.
pub const NET_PROTOCOL_PREFIX: &'static str = "chud_";

/// The binary request response protocol name.
pub const RR_PROTOCOL_PREFIX: &'static str = "/chud_bin";

/// The number of milliseconds to wait between synchronizing with peers.
pub const SYNCHRONIZATION_INTERVAL: u64 = 120000;

/// The number of milliseconds before a synchronization request is invalid.
pub const SYNCHRONIZATION_TIMEOUT: u64 = 30000;

/// The name of the floodsub topic to which new messages are published.
pub const FLOODSUB_MESSAGE_TOPIC: &'static str = "new_msg";
