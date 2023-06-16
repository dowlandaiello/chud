use chud::{
	net::client::Client,
	rpc::cmd::Cmd,
	sys::msg::{Message, MessageData},
};
use std::error::Error;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_client_load_write() -> Result<(), Box<dyn Error>> {
	let mut client = Client::new(0);

	let data = MessageData::new(Vec::new(), None, String::from(""), 0);
	let msg = Message::try_from(data)?;

	// Insert the message
	client.runtime.insert_message(msg);

	// Ensure read and written clients are the same
	client
		.write_to_disk()
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e.message()))?;
	let client2 = Client::load_from_disk(0)
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e.message()))?;

	assert_eq!(client.runtime, client2.runtime);

	Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_client_start() -> Result<(), Box<dyn Error>> {
	let (tx, rx) = async_channel::unbounded();
	let (tx_resp, _) = async_channel::unbounded();
	tx.send(Cmd::Terminate).await?;

	let mut client = Client::new(0);
	client.start(rx, tx_resp, Vec::new()).await?;

	Ok(())
}
