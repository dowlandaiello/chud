use chud::{
	net::client::{DefaultClient as Client, NetworkClient},
	rpc::cmd::Cmd,
	sys::{
		msg::{Message, MessageData},
		rt::Rt,
	},
};
use std::error::Error;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen_test]
async fn test_client_start() -> Result<(), Box<dyn Error>> {
	let (tx, rx) = async_channel::unbounded();
	let (tx_resp, _) = async_channel::unbounded();
	tx.send(Cmd::Terminate).await?;

	let mut client = Client::default();
	client
		.start(rx, tx_resp, <Vec<String>>::new(), None, Vec::new(), None)
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
	chud::start(1, Vec::new());
	let msg_data = MessageData::new(Vec::new(), None, None, None, 0, 0);
	let msg_data_js = serde_wasm_bindgen::to_value(&msg_data)
		.map_err(|e| <serde_wasm_bindgen::Error as Into<Box<dyn Error>>>::into(e))?;
	chud::flush()
		.await
		.map_err(|e| <String as Into<Box<dyn Error>>>::into(e))?;
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
