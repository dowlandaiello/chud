use super::req::{Request, Response};
use libp2p::{
	floodsub::Floodsub,
	identify::Behaviour,
	kad::{record::store::MemoryStore, Kademlia},
	swarm::NetworkBehaviour,
};
use libp2p_request_response::cbor::Behaviour as RRBehavior;

/// Network behavior combining floodsub, KAD DHT, and identify network behaviors.
#[derive(NetworkBehaviour)]
pub struct Behavior {
	kad: Kademlia<MemoryStore>,
	floodsub: Floodsub,
	identify: Behaviour,
	rresponse: RRBehavior<Request, Response>,
}

impl Behavior {
	/// Creates a new network behavior with the given components.
	pub fn new(
		kad: Kademlia<MemoryStore>,
		floodsub: Floodsub,
		identify: Behaviour,
		request_response: RRBehavior<Request, Response>,
	) -> Self {
		Self {
			kad,
			floodsub,
			identify,
			rresponse: request_response,
		}
	}

	/// Gets a reference to the DHT backing the behavior.
	pub fn kad(&self) -> &Kademlia<MemoryStore> {
		&self.kad
	}

	/// Gets a mutable reference to the DHT backing the behavior.
	pub fn kad_mut(&mut self) -> &mut Kademlia<MemoryStore> {
		&mut self.kad
	}

	/// Gets a reference to the floodsub backing the behavior.
	pub fn floodsub(&self) -> &Floodsub {
		&self.floodsub
	}

	/// Gets a mutable reference to the floodsub backing the behavior.
	pub fn floodsub_mut(&mut self) -> &mut Floodsub {
		&mut self.floodsub
	}

	/// Gets a reference to the identify network behavior backing the behavior.
	pub fn identify(&self) -> &Behaviour {
		&self.identify
	}

	/// Gets a mutable reference to the identify network behavior backing the behavior.
	pub fn identify_mut(&mut self) -> &mut Behaviour {
		&mut self.identify
	}

	/// Gets a reference to the request-response network behavior backing the behavior.
	pub fn request_response(&self) -> &RRBehavior<Request, Response> {
		&self.rresponse
	}

	/// Gets a mutable reference to the request-response network behavior backing the behavior.
	pub fn request_response_mut(&mut self) -> &mut RRBehavior<Request, Response> {
		&mut self.rresponse
	}
}
