use chud::{
	net::client::Client,
	sys::msg::{Message, MessageData},
};
use std::error::Error;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
async fn test_load_write() -> Result<(), Box<dyn Error>> {
	let mut client = Client::default();

	let data = MessageData::new(Vec::new(), None, String::from(""), 0);
	let msg = Message::try_from(data)?;

	// Insert the message
	client.runtime.insert_message(msg);

	// Ensure read and written clients are the same
	client
		.write_to_disk()
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e.message()))?;
	let client2 = Client::load_from_disk()
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e.message()))?;

	assert_eq!(client.runtime, client2.runtime);

	Ok(())
}
