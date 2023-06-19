macro_rules! nonfatal {
	($e:expr,$msg:expr) => {{
		let res = $e;
		match res {
			Ok(v) => v,
			Err(e) => {
				error!($msg, e);
				continue;
			}
		}
	}};
}

pub(crate) use nonfatal;
