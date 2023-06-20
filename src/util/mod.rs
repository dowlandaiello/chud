macro_rules! nonfatal {
	($e:expr,$req_id:expr,$resp_tx:ident) => {{
		let res = $e;
		match res {
			Ok(v) => v,
			Err(e) => {
				$resp_tx
					.send(CmdResp::Error {
						error: e.to_string(),
						req_id: $req_id,
					})
					.await
					.expect("Channel send failed.");
				continue;
			}
		}
	}};
}

pub(crate) use nonfatal;
