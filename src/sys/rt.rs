use super::{super::crypto::hash::Hash, msg::Message};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A caching layer for the underlying DHT of messages in the CHUD network.
#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Rt {
	messages: HashMap<Hash, Message>,
	chains: Vec<Chain>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Chain {
	head: Hash,
	messages: HashSet<Hash>,
}

impl Rt {
	/// Registers the message in the runtime, updating the consensus view if need be.
	/// Assumes the message is valid per consensus rules.
	pub fn insert_message(&mut self, msg: Message) {
		self.update_head(msg.hash(), msg.data().prev());
		self.messages.insert(msg.hash(), msg);
	}

	/// Advances the head that is the previous node referenced by the message, or else
	/// inserts the message as a new head.
	fn update_head(&mut self, new_head: Hash, prev: Option<Hash>) {
		// Update head that is prev to the new message,
		// or else insert the message as a new head
		for chain in self.chains.iter_mut() {
			if let Some(prev) = prev {
				if prev == chain.head {
					chain.head = new_head;
					chain.messages.insert(new_head);

					return;
				}
			}
		}

		self.chains.push(Chain {
			head: new_head,
			messages: HashSet::from([new_head]),
		});
	}

	/// Determines the longest chain in the runtime, returning None if no chains exist.
	pub fn longest_chain(&self) -> Option<Hash> {
		let mut heads = self
			.chains
			.as_slice()
			.into_iter()
			.filter_map(|chain| self.messages.get(&chain.head))
			.collect::<Vec<&Message>>();

		heads.sort_by(|a, b| usize::cmp(&b.data().height(), &a.data().height()));
		heads.get(0).map(|msg| msg.hash())
	}

	/// Gets the message in the current chain with the indicated hash.
	/// Returns None if the message does not exist, or if it is not in
	/// the current chain.
	pub fn get_message(&self, hash: Hash) -> Option<&Message> {
		let chain = self
			.longest_chain()
			.and_then(|hash| self.chains.iter().find(|chain| chain.head == hash))?;

		if !chain.messages.contains(&hash) {
			return None;
		};

		self.messages.get(&hash)
	}

	/// Gets the head of the current blockchain.
	pub fn head(&self) -> Option<&Message> {
		let longest = self.longest_chain()?;
		self.messages.get(&longest)
	}
}

#[cfg(test)]
mod tests {
	use super::{super::msg::MessageData, *};
	use std::error::Error;

	#[test]
	fn test_default() {
		let rt = Rt::default();
		assert_eq!(rt.messages.len(), 0);
		assert_eq!(rt.chains.len(), 0);
	}

	#[test]
	fn test_insert_message() -> Result<(), Box<dyn Error>> {
		let mut rt = Rt::default();

		// Generate blank message
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg = Message::try_from(data)?;

		// Insert the message
		rt.insert_message(msg.clone());

		assert_eq!(
			rt.chains
				.get(0)
				.ok_or("Message not found in head list.")?
				.head,
			msg.hash()
		);
		assert!(rt
			.chains
			.get(0)
			.ok_or("Message not found in head list.")?
			.messages
			.contains(&msg.hash()));
		assert_eq!(
			rt.messages
				.get(&msg.hash())
				.ok_or("Message not found in messages.")?,
			&msg
		);

		Ok(())
	}

	#[test]
	fn test_longest_chain() -> Result<(), Box<dyn Error>> {
		let mut rt = Rt::default();

		// Generate blank message
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg = Message::try_from(data)?;

		// Insert the message
		rt.insert_message(msg.clone());

		assert_eq!(
			rt.longest_chain().ok_or("No longest chain found")?,
			msg.hash()
		);

		// Make another chain
		let data2 = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg2 = Message::try_from(data2)?;
		rt.insert_message(msg2.clone());

		let data3 = MessageData::new(Vec::new(), Some(msg2.hash()), String::from(""), 1);
		let msg3 = Message::try_from(data3)?;
		rt.insert_message(msg3.clone());

		assert_eq!(
			rt.longest_chain().ok_or("No longest chain found")?,
			msg3.hash()
		);
		assert_eq!(rt.chains.len(), 2);

		Ok(())
	}

	#[test]
	fn test_get_message() -> Result<(), Box<dyn Error>> {
		let mut rt = Rt::default();

		assert!(rt.get_message([0; 32]).is_none());

		// Generate blank message
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg = Message::try_from(data)?;

		// Insert the message
		rt.insert_message(msg.clone());

		assert_eq!(rt.get_message(msg.hash()), Some(&msg));

		// Make another chain
		let data2 = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg2 = Message::try_from(data2)?;
		rt.insert_message(msg2.clone());

		let data3 = MessageData::new(Vec::new(), Some(msg2.hash()), String::from(""), 1);
		let msg3 = Message::try_from(data3)?;
		rt.insert_message(msg3.clone());

		assert_eq!(rt.get_message(msg.hash()), None);
		assert_eq!(rt.get_message(msg2.hash()), Some(&msg2));
		assert_eq!(rt.get_message(msg3.hash()), Some(&msg3));

		Ok(())
	}

	#[test]
	fn test_head() -> Result<(), Box<dyn Error>> {
		let mut rt = Rt::default();

		// Generate blank message
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg = Message::try_from(data)?;

		// Insert the message
		rt.insert_message(msg.clone());

		assert_eq!(rt.head(), Some(&msg));

		// Make another chain
		let data2 = MessageData::new(Vec::new(), None, String::from(""), 0);
		let msg2 = Message::try_from(data2)?;
		rt.insert_message(msg2.clone());

		let data3 = MessageData::new(Vec::new(), Some(msg2.hash()), String::from(""), 1);
		let msg3 = Message::try_from(data3)?;
		rt.insert_message(msg3.clone());

		assert_eq!(rt.head(), Some(&msg3));

		Ok(())
	}
}
