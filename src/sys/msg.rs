use super::super::{captcha::Captcha, crypto::hash::Hash};
use serde::{Deserialize, Serialize};
use serde_json::Error;

/// A message in the CHUD blockchain. Primarily constituted by arbitrary data,
/// and newly generated and previous captcha answers.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct MessageData {
	data: Vec<u8>,
	prev: Option<Hash>,
	new_captcha: Captcha,
	captcha_ans: String,
	height: usize,
}

impl MessageData {
	/// Constructs a new message in the context of a greater blockchain. Expects a height and answer to the
	/// derived corresponding captcha, as well as a previous message, and arbitrary data. Generates a new
	/// captcha to attach to the message.
	pub fn new(data: Vec<u8>, prev: Option<Hash>, captcha_ans: String, height: usize) -> Self {
		Self {
			data,
			prev,
			new_captcha: Captcha::default(),
			captcha_ans,
			height,
		}
	}

	/// Gets a reference to the data in the message.
	pub fn data(&self) -> &[u8] {
		self.data.as_slice()
	}

	/// Gets hash of the previous message in the chain.
	pub fn prev(&self) -> Option<Hash> {
		self.prev
	}

	/// Gets a reference to the new generated captcha contained in the message.
	pub fn new_captcha(&self) -> &Captcha {
		&self.new_captcha
	}

	/// Gets a reference to the answer to the derived captcha from the message.
	pub fn captcha_ans(&self) -> &str {
		self.captcha_ans.as_str()
	}

	/// Gets the index of the message in the chain. Should be prev.height + 1.
	pub fn height(&self) -> usize {
		self.height
	}
}

/// A message in the CHUD blockchain. Primarily constituted by arbitrary data,
/// and newly generated and previous captcha answers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Message {
	data: MessageData,
	hash: Hash,
}

impl TryFrom<MessageData> for Message {
	type Error = Error;

	fn try_from(data: MessageData) -> Result<Self, Self::Error> {
		let encoded = serde_json::to_vec(&data)?;
		let hashed = blake3::hash(encoded.as_slice());

		Ok(Self {
			data,
			hash: hashed.into(),
		})
	}
}

impl Message {
	/// Gets a reference to the body of the message.
	pub fn data(&self) -> &MessageData {
		&self.data
	}

	/// Gets the hash of the message.
	pub fn hash(&self) -> Hash {
		self.hash
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new() {
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		assert_eq!(data.data, <Vec<u8>>::new());
		assert_eq!(data.prev, None);
		assert_eq!(data.captcha_ans, String::from(""));
	}

	#[test]
	fn test_try_from() -> Result<(), Error> {
		let data = MessageData::new(Vec::new(), None, String::from(""), 0);
		let _ = Message::try_from(data)?;

		Ok(())
	}
}
