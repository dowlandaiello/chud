use super::crypto::hash::Hash;
use captcha_rs::CaptchaBuilder;
use serde::{Deserialize, Serialize};

/// A captcha to be solved. Constituted by base64-encoded image data, and a hashed answer.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct Captcha {
	data: Vec<u8>,
	answer: Hash,
}

impl Default for Captcha {
	// Constructs a captcha, returning its base64 data and hashed answer.
	fn default() -> Self {
		let captcha = CaptchaBuilder::new()
			.length(5)
			.width(130)
			.height(40)
			.dark_mode(true)
			.complexity(8)
			.compression(50)
			.build();

		Self {
			data: captcha.to_base64().into_bytes(),
			answer: blake3::hash(captcha.text.as_bytes()).into(),
		}
	}
}

impl Captcha {
	/// Gets a slice of the image data underlying the captcha.
	pub fn data(&self) -> &[u8] {
		self.data.as_slice()
	}

	/// Gets a reference to the hash of the answer to the captcha.
	pub fn answer(&self) -> &Hash {
		&self.answer
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_default() {
		let captcha = Captcha::default();
		assert_eq!(captcha.data.is_empty(), false);
		assert_ne!(*captcha.answer, [0; 32]);
	}
}
