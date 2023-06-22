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

	let data = MessageData::new(Vec::new(), None, None, None, 0, 0);
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
	client
		.start(rx, tx_resp, Vec::new(), None, Vec::new())
		.await?;

	Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_start() -> Result<(), Box<dyn Error>> {
	chud::start(0, Vec::new());

	Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_submit_message() -> Result<(), Box<dyn Error>> {
	chud::start(0, Vec::new());
	let msg_data =
		serde_wasm_bindgen::to_value(&MessageData::new(Vec::new(), None, None, None, 0, 0))
			.map_err(|e| <serde_wasm_bindgen::Error as Into<Box<dyn Error>>>::into(e))?;
	chud::submit_message(msg_data)
		.await
		.map_err(|e| e.into())
		.map(|_| ())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_load_message() -> Result<(), Box<dyn Error>> {
	chud::start(0, Vec::new());
	let msg_data = MessageData::new(Vec::new(), None, None, None, 0, 0);
	let msg_data_js = serde_wasm_bindgen::to_value(&msg_data)
		.map_err(|e| <serde_wasm_bindgen::Error as Into<Box<dyn Error>>>::into(e))?;
	let hash = chud::submit_message(msg_data_js)
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e))?;
	let msg_js = chud::load_message(hash.as_str())
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e))?;
	let msg: Message = serde_wasm_bindgen::from_value(msg_js)
		.map_err(|e| <serde_wasm_bindgen::Error as Into<Box<dyn Error>>>::into(e))?;
	assert_eq!(hash, hex::encode(msg.data().hashed().map_err(Box::new)?));

	Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_get_head() -> Result<(), Box<dyn Error>> {
	chud::start(0, Vec::new());
	let msg_data = MessageData::new(Vec::new(), None, None, None, 0, 0);
	let msg_data_js = serde_wasm_bindgen::to_value(&msg_data)
		.map_err(|e| <serde_wasm_bindgen::Error as Into<Box<dyn Error>>>::into(e))?;
	let hash = chud::submit_message(msg_data_js)
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e))?;
	let head = chud::get_head()
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e))?;
	assert_eq!(hash, head);

	Ok(())
}
