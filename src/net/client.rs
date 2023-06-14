use super::super::sys::rt::Rt;
use indexed_db_futures::{
	idb_transaction::IdbTransaction, prelude::IdbTransactionMode, request::IdbOpenDbRequestLike,
	IdbDatabase, IdbQuerySource, IdbVersionChangeEvent,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::DomException;

/// The name of the indexed db in which chud data is stored.
pub const DB_NAME: &'static str = "chud_db";

/// The object store in which runtime data is stored.
pub const RUNTIME_STORE: &'static str = "runtime";

/// The key under the runtime store under which the state is stored.
pub const STATE_KEY: &'static str = "state";

/// An interface with the CHUD network.
#[derive(Default, Serialize, Deserialize)]
pub struct Client {
	pub runtime: Rt,
}

impl Client {
	/// Loads the saved blockchain data from indexeddb.
	pub async fn load_from_disk() -> Result<Self, DomException> {
		let mut client = Client::default();

		// Create the object store if it doesn't exist
		let mut db_req = IdbDatabase::open(DB_NAME)?;
		db_req.set_on_upgrade_needed(Some(|e: &IdbVersionChangeEvent| -> Result<(), JsValue> {
			if let None = e.db().object_store_names().find(|n| n == RUNTIME_STORE) {
				e.db().create_object_store(RUNTIME_STORE)?;
			}

			Ok(())
		}));

		// Open the database
		let db: IdbDatabase = db_req.into_future().await?;

		// Read the state from the database
		let tx: IdbTransaction =
			db.transaction_on_one_with_mode(RUNTIME_STORE, IdbTransactionMode::Readonly)?;
		let store = tx.object_store(RUNTIME_STORE)?;

		if let Some(Ok(record)) = store.get_owned(STATE_KEY)?.await.and_then(|val| {
			Ok(val.map(|val| {
				serde_wasm_bindgen::from_value(val)
					.map_err(|e| DomException::from(JsValue::from_str(format!("{}", e).as_str())))
			}))
		})? {
			client = record;
		}

		Ok(client)
	}

	/// Writes the blockchain to indexeddb.
	pub async fn write_to_disk(&self) -> Result<(), DomException> {
		// Create the object store if it doesn't exist
		let mut db_req = IdbDatabase::open(DB_NAME)?;
		db_req.set_on_upgrade_needed(Some(|e: &IdbVersionChangeEvent| -> Result<(), JsValue> {
			if let None = e.db().object_store_names().find(|n| n == RUNTIME_STORE) {
				e.db().create_object_store(RUNTIME_STORE)?;
			}

			Ok(())
		}));

		// Open the database
		let db: IdbDatabase = db_req.into_future().await?;

		// Write the state to the database
		let tx: IdbTransaction =
			db.transaction_on_one_with_mode(RUNTIME_STORE, IdbTransactionMode::Readwrite)?;
		let store = tx.object_store(RUNTIME_STORE)?;

		store.put_key_val_owned(
			STATE_KEY,
			&serde_wasm_bindgen::to_value(self)
				.map_err(|e| DomException::from(JsValue::from_str(format!("{}", e).as_str())))?,
		)?;

		Ok(())
	}
}
