use libp2p::{
	floodsub::Floodsub,
	identify::Behaviour,
	kad::{record::store::MemoryStore, Kademlia},
	swarm::NetworkBehaviour,
};

/// Network behavior combining floodsub, KAD DHT, and identify network behaviors.
#[derive(NetworkBehaviour)]
pub struct Behavior {
	kad: Kademlia<MemoryStore>,
	floodsub: Floodsub,
	identify: Behaviour,
}

impl Behavior {
	/// Creates a new network behavior with the given components.
	pub fn new(kad: Kademlia<MemoryStore>, floodsub: Floodsub, identify: Behaviour) -> Self {
		Self {
			kad,
			floodsub,
			identify,
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
}
