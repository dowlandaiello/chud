use serde::{
	de::{self, Visitor},
	Deserialize, Deserializer, Serialize, Serializer,
};
use std::{fmt, ops::Deref};

/// A hex-encoded, non-0x padded SHA-256 hash.
#[derive(Hash, PartialEq, Eq, Debug, Clone)]
pub struct Hash {
	bytes: [u8; 32],
}

impl From<blake3::Hash> for Hash {
	fn from(h: blake3::Hash) -> Self {
		Self { bytes: h.into() }
	}
}

impl From<[u8; 32]> for Hash {
	fn from(h: [u8; 32]) -> Self {
		Self { bytes: h }
	}
}

impl Deref for Hash {
	type Target = [u8; 32];
	fn deref(&self) -> &[u8; 32] {
		&self.bytes
	}
}

impl AsRef<[u8]> for Hash {
	fn as_ref(&self) -> &[u8] {
		self.bytes.as_slice()
	}
}

impl Serialize for Hash {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(hex::encode(self.bytes).as_str())
	}
}

struct HashVisitor;

impl<'de> Visitor<'de> for HashVisitor {
	type Value = Hash;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("a hexdadecimal string")
	}

	fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
	where
		E: de::Error,
	{
		let bytes = hex::decode(value).map_err(de::Error::custom)?;
		let arr: [u8; 32] = bytes
			.try_into()
			.map_err(|_| "invalid hash")
			.map_err(de::Error::custom)?;

		Ok(Hash { bytes: arr })
	}
}

impl<'de> Deserialize<'de> for Hash {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(HashVisitor)
	}
}
