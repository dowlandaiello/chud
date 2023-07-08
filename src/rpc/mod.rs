#[cfg(not(target_arch = "wasm32"))]
use actix_web::{
	get, post,
	web::{Data, Json},
	HttpResponse, HttpResponseBuilder, Responder,
};
use async_channel::{Receiver, Sender};
use cmd::{Cmd, CmdResp, LoadMsgReq, SubmitMsgReq};
use std::error::Error;

pub mod cmd;

/// Stops the running client.
#[cfg(not(target_arch = "wasm32"))]
#[post("/terminate")]
pub async fn terminate(
	cmd_tx: Data<Sender<Cmd>>,
	_resp_rx: Data<Receiver<CmdResp>>,
) -> impl Responder {
	cmd_tx.send(Cmd::Terminate).await?;

	Ok::<HttpResponseBuilder, Box<dyn Error>>(HttpResponse::Ok())
}

/// Checks that the RPC connection is valid.
#[cfg(not(target_arch = "wasm32"))]
#[get("/health_check")]
pub async fn health_check() -> impl Responder {
	HttpResponse::Ok()
}

/// Submits a message to the network.
#[cfg(not(target_arch = "wasm32"))]
#[post("/submit_msg")]
pub async fn submit_msg(
	cmd_tx: Data<Sender<Cmd>>,
	resp_rx: Data<Receiver<CmdResp>>,
	Json(json): Json<SubmitMsgReq>,
) -> impl Responder {
	let req_id = instant::now() as usize;

	cmd_tx.send(Cmd::SubmitMsg { req_id, req: json }).await?;

	loop {
		match resp_rx.recv().await? {
			CmdResp::MsgSubmitted {
				hash: h,
				req_id: resp_id,
			} => {
				if resp_id == req_id {
					return Ok(HttpResponse::Ok().body(hex::encode(h)));
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if req_id == resp_id {
					return Err(<Box<dyn Error>>::from(
						format!("Failed to submit the message: {}", error).as_str(),
					));
				}
			}
			_ => continue,
		}
	}
}

/// Reads a message from the network.
#[cfg(not(target_arch = "wasm32"))]
#[get("/load_msg")]
pub async fn load_msg(
	cmd_tx: Data<Sender<Cmd>>,
	resp_rx: Data<Receiver<CmdResp>>,
	Json(json): Json<LoadMsgReq>,
) -> impl Responder {
	let req_id = instant::now() as usize;

	cmd_tx.send(Cmd::LoadMsg { req_id, req: json }).await?;

	loop {
		match resp_rx.recv().await? {
			CmdResp::MsgLoaded {
				msg,
				req_id: resp_id,
			} => {
				if resp_id == req_id {
					return Ok(HttpResponse::Ok().json(msg));
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if req_id == resp_id {
					return Err(<Box<dyn Error>>::from(
						format!("Failed to load the message: {}", error).as_str(),
					));
				}
			}
			_ => continue,
		}
	}
}

/// Gets the hash of the head.
#[cfg(not(target_arch = "wasm32"))]
#[get("/get_head")]
pub async fn get_head(
	cmd_tx: Data<Sender<Cmd>>,
	resp_rx: Data<Receiver<CmdResp>>,
) -> impl Responder {
	let req_id = instant::now() as usize;

	cmd_tx.send(Cmd::GetHead { req_id }).await?;

	loop {
		match resp_rx.recv().await? {
			CmdResp::HeadLoaded {
				hash,
				req_id: resp_id,
			} => {
				if resp_id == req_id {
					return Ok(HttpResponse::Ok().body(hex::encode(hash)));
				}
			}
			CmdResp::Error {
				error,
				req_id: resp_id,
			} => {
				if req_id == resp_id {
					return Err(<Box<dyn Error>>::from(
						format!("Failed to load the head: {}", error).as_str(),
					));
				}
			}
			_ => continue,
		}
	}
}
