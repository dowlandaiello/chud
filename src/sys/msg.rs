use super::{
	super::{captcha::Captcha, crypto::hash::Hash},
	CAPTCHA_ANS_LOOKBACK_FACTOR,
};
use serde::{Deserialize, Serialize};
use serde_json::Error;

/// A message in the CHUD blockchain. Primarily constituted by arbitrary data,
/// and newly generated and previous captcha answers.
#[derive(Serialize, Hash, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct MessageData {
	data: Vec<u8>,
	prev: Option<Hash>,
	new_captcha: Captcha,
	captcha_ans: Option<String>,
	captcha_src: Option<Hash>,
	height: usize,
	timestamp: u128,
}

impl MessageData {
	/// Constructs a new message in the context of a greater blockchain. Expects a height and answer to the
	/// derived corresponding captcha, as well as a previous message, and arbitrary data. Generates a new
	/// captcha to attach to the message.
	pub fn new(
		data: Vec<u8>,
		prev: Option<Hash>,
		captcha_ans: Option<String>,
		captcha_src: Option<Hash>,
		height: usize,
		timestamp: u128,
	) -> Self {
		Self {
			data,
			prev,
			new_captcha: Captcha::default(),
			captcha_ans,
			captcha_src,
			height,
			timestamp,
		}
	}

	/// Gets a reference to the data in the message.
	pub fn data(&self) -> &[u8] {
		self.data.as_slice()
	}

	/// Gets hash of the previous message in the chain.
	pub fn prev(&self) -> Option<&Hash> {
		self.prev.as_ref()
	}

	/// Gets a reference to the new generated captcha contained in the message.
	pub fn new_captcha(&self) -> &Captcha {
		&self.new_captcha
	}

	/// Gets a reference to the answer to the derived captcha from the message.
	pub fn captcha_ans(&self) -> Option<&str> {
		self.captcha_ans.as_deref()
	}

	/// Gets a reference to the hash of the message from which the captcha is derived that this message is answering.
	pub fn captcha_src(&self) -> Option<&Hash> {
		self.captcha_src.as_ref()
	}

	/// Gets the index of the message in the chain. Should be prev.height + 1.
	pub fn height(&self) -> usize {
		self.height
	}

	/// Gets the UNIX timestamp of the message.
	pub fn timestamp(&self) -> u128 {
		self.timestamp
	}

	/// Calculates the hash of the transaction data.
	pub fn hashed(&self) -> Result<Hash, Error> {
		let encoded = serde_json::to_vec(&self)?;
		Ok(blake3::hash(encoded.as_slice()).into())
	}

	/// Calculates the hash of the transaction whose captcha this transaction
	/// is answering.
	pub fn lookback(&self) -> Option<u64> {
		// Calculate the number of posts back to look for the answer based on
		// the hash of the message
		let h = self.hashed().ok()?;
		let digits = hex::encode(h);
		let relevant_digits = &digits[..CAPTCHA_ANS_LOOKBACK_FACTOR];

		let lookback_bytes: [u8; 8] = hex::decode(relevant_digits).ok()?.try_into().ok()?;
		Some(u64::from_le_bytes(lookback_bytes))
	}
}

/// A message in the CHUD blockchain. Primarily constituted by arbitrary data,
/// and newly generated and previous captcha answers.
#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Message {
	data: MessageData,
	hash: Hash,
}

impl TryFrom<MessageData> for Message {
	type Error = Error;

	fn try_from(data: MessageData) -> Result<Self, Self::Error> {
		let hash = data.hashed()?;
		Ok(Self { data, hash })
	}
}

impl Message {
	/// Gets a reference to the body of the message.
	pub fn data(&self) -> &MessageData {
		&self.data
	}

	/// Gets the hash of the message.
	pub fn hash(&self) -> &Hash {
		&self.hash
	}

	/// Deserializes the data inside the message to some deserializable type.
	pub fn as_data<'a, T: Deserialize<'a>>(&'a self) -> Result<T, Error> {
		serde_json::from_slice(self.data().data())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_new() {
		let data = MessageData::new(Vec::new(), None, None, None, 0, 0);
		assert_eq!(data.data, <Vec<u8>>::new());
		assert_eq!(data.prev, None);
		assert_eq!(data.captcha_ans, None);
		assert_eq!(data.timestamp, 0);
	}

	#[test]
	fn test_try_from() -> Result<(), Error> {
		let data = MessageData::new(Vec::new(), None, None, None, 0, 0);
		let _ = Message::try_from(data)?;

		Ok(())
	}
}
