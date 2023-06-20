use super::{
	super::{
		crypto::hash::Hash,
		sys::{msg::Message, rt::Rt},
	},
	behavior::BehaviorEvent,
	FLOODSUB_MESSAGE_TOPIC,
};
use libp2p::floodsub::{Floodsub, FloodsubEvent, Topic};
use serde_json::Error as SerdeError;
use std::{
	error::Error as StdError,
	fmt::{Display, Error as FmtError, Formatter},
};

/// Events emitted by the message behavior
#[derive(Debug)]
pub enum Event {
	/// Emitted when a message gets received
	MessageReceived(Hash),
}

/// Errors emitted by the message behavior
#[derive(Debug)]
pub enum Error {
	SerializationError(SerdeError),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
		match self {
			Self::SerializationError(e) => {
				write!(f, "encountered an error in serialization: {}", e)
			}
		}
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Self::SerializationError(e) => Some(e),
		}
	}
}

impl From<SerdeError> for Error {
	fn from(e: SerdeError) -> Self {
		Self::SerializationError(e)
	}
}

/// A context that handles swarm events dealing with messages.
#[derive(Default)]
pub struct Context;

impl Context {
	pub fn poll(
		&mut self,
		rt: &mut Rt,
		floodsub: &mut Floodsub,
		in_event: Option<BehaviorEvent>,
	) -> (Result<Option<Event>, Error>, Option<BehaviorEvent>) {
		match in_event {
			// Possible floodsub message topics:
			// - new_msg
			Some(BehaviorEvent::Floodsub(FloodsubEvent::Message(fs_msg))) => {
				// TODO: consensus
				// - Ensure hash is valid
				// - Ensure timestamp is valid
				// A new message has been received
				if fs_msg.topics.contains(&Topic::new(FLOODSUB_MESSAGE_TOPIC)) {
					if let Ok(msg) = serde_json::from_slice::<Message>(&fs_msg.data) {
						let hash = msg.hash().clone();
						rt.insert_message(msg);

						return (Ok(Some(Event::MessageReceived(hash))), None);
					} else {
						(Ok(None), None)
					}
				} else {
					(Ok(None), None)
				}
			}
			_ => (Ok(None), in_event),
		}
	}

	/// Publishes a message to the floodsub messages topic.
	pub fn submit_message(&mut self, msg: Message, floodsub: &mut Floodsub) -> Result<(), Error> {
		let serialized = serde_json::to_vec(&msg)?;
		floodsub.publish(Topic::new(FLOODSUB_MESSAGE_TOPIC), serialized);

		Ok(())
	}
}
