use super::{
	super::{
		crypto::hash::Hash,
		sys::{msg::Message, rt::Rt},
	},
	behavior::BehaviorEvent,
	req::{Request, Response},
	DHT_QUORUM, SAMPLING_SIZE, SYNCHRONIZATION_TIMEOUT,
};
use instant::{Duration, Instant};
use libp2p::{
	kad::{
		record::store::Error as KadError, store::MemoryStore, GetRecordOk, Kademlia, KademliaEvent,
		QueryId, QueryResult, Record, RecordKey,
	},
	request_response::{cbor::Behaviour as RRBehavior, Event as RREvent, Message as RRMessage},
	PeerId,
};
use serde_json::Error as SerdeError;
use std::{
	collections::{HashMap, HashSet},
	error::Error as StdError,
	fmt::{Display, Error as FmtError, Formatter},
};

/// Events emitted by the synchronization behavior
#[derive(Debug)]
pub enum Event {
	/// Emitted when the entire blockchain has been committed
	MessageCommitted(Hash),

	/// Emitted when a message is successfully loaded
	MessageLoaded(Message),

	/// Emitted when the longest chain has been updated
	LongestChainUpdated {
		/// The hash of the HEAD of the longest chain
		hash: Hash,

		// The length of the longest chain
		height: usize,
	},
}

/// Any error that may occur while synchronizing the blockchain.
#[derive(Debug)]
pub enum Error {
	MissingMessage,
	SerializationError(SerdeError),
	KadError(KadError),
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
		match self {
			Error::MissingMessage => write!(f, "the runtime is missing the requested message"),
			Error::SerializationError(e) => write!(f, "serialization error while syncing: {}", e),
			Error::KadError(e) => write!(f, "DHT error: {}", e),
		}
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		match self {
			Error::MissingMessage => None,
			Error::SerializationError(e) => Some(e),
			Error::KadError(e) => Some(e),
		}
	}
}

impl From<KadError> for Error {
	fn from(e: KadError) -> Self {
		Self::KadError(e)
	}
}

/// A NetworkBehavior implementing synchronization utilities including:
/// - uploading the blockchain
/// - downloading the blockchain
#[derive(Default)]
pub struct Context {
	// Questioning rounds for synchronization of the longest chain hash
	chain_downloads: Vec<SynchronizationRequest>,

	// Upload requests
	chain_uploads: HashMap<QueryId, Hash>,

	// Download requests
	message_downloads: HashSet<QueryId>,
}

// The state of a round of questioning regarding the longest chain
struct SynchronizationRequest {
	initiated_at: Instant,
	peers_contacted: Vec<PeerId>,
	results: Vec<Response>,
}

impl Context {
	/// Checks the status of requested operations on the context, and returns
	/// the appropriate event if an operation was completed.
	pub fn poll(
		&mut self,
		rt: &Rt,
		request_response: &mut RRBehavior<Request, Response>,
		in_event: Option<BehaviorEvent>,
	) -> (Result<Option<Event>, Error>, Option<BehaviorEvent>) {
		match in_event {
			Some(BehaviorEvent::Kad(e)) => match e {
				KademliaEvent::OutboundQueryProgressed { id, result, .. } => {
					// If the user requested to upload the chain, notify them that
					// it was successful
					if let Some(msg_hash) = self.chain_uploads.get(&id).cloned() {
						self.chain_uploads.remove(&id);

						if let QueryResult::PutRecord(Ok(_)) = result {
							return (Ok(Some(Event::MessageCommitted(msg_hash.clone()))), None);
						}
					}

					// We previously requested to download a message.
					// Use the according event type
					if self.message_downloads.contains(&id) {
						// We successfully found the message
						if let QueryResult::GetRecord(Ok(GetRecordOk::FoundRecord(record))) = result
						{
							if let Ok(msg) = serde_json::from_slice(record.record.value.as_slice())
							{
								// Notify the user that the message was found
								return (Ok(Some(Event::MessageLoaded(msg))), None);
							}
						}
					}

					(Ok(None), None)
				}
				_ => (Ok(None), None),
			},
			Some(BehaviorEvent::Rresponse(e)) => match e {
				RREvent::Message { message, .. } => match message {
					RRMessage::Request {
						request, channel, ..
					} => match request {
						// A peer asked for the hash of the longest chain. Find it, and if it
						// exists, respond
						Request::LongestChain { query_round } => {
							if let Some(longest_chain) =
								rt.longest_chain().and_then(|hash| rt.get_message(hash))
							{
								let _ = request_response.send_response(
									channel,
									Response::LongestChain {
										hash: longest_chain.hash().clone(),
										height: longest_chain.data().height(),
										query_round,
									},
								);
							}

							(Ok(None), None)
						}
					},
					RRMessage::Response { response, .. } => match response {
						// A peer gave us the longest chain.
						// Add it to the list of results, and
						// if we have passed 50% of sampled
						// peers, emit the event
						Response::LongestChain {
							hash,
							height,
							query_round,
						} => {
							if let Some(query_data) = self.chain_downloads.get_mut(query_round) {
								// Ensure that the query has not expired
								if Instant::now() - query_data.initiated_at
									< Duration::from_millis(SYNCHRONIZATION_TIMEOUT)
								{
									query_data.results.push(Response::LongestChain {
										hash,
										height,
										query_round,
									});

									// Check if we have passed quorum, and if we have, notify the user
									// of the new HEAD
									if query_data.results.len() as f32
										> query_data.peers_contacted.len() as f32 * SAMPLING_SIZE
									{
										// Get the response with the longest chain
										let mut longest_chain = query_data
											.results
											.iter()
											.filter_map(|msg| match msg {
												Response::LongestChain { hash, height, .. } => {
													Some((hash.clone(), height.clone()))
												}
											})
											.collect::<Vec<(Hash, usize)>>();
										longest_chain.sort_by_key(|msg| msg.1);

										let (hash, height) =
											longest_chain.remove(longest_chain.len() - 1);

										return (
											Ok(Some(Event::LongestChainUpdated { height, hash })),
											None,
										);
									}
								}
							}

							(Ok(None), None)
						}
					},
				},
				_ => (Ok(None), None),
			},
			_ => (Ok(None), in_event),
		}
	}

	/// Commits all transactions in the client's blockchain to the DHT.
	pub fn upload_chain(&mut self, rt: &Rt, kad: &mut Kademlia<MemoryStore>) -> Result<(), Error> {
		// Only continue synchronizing if some chain data exists
		let longest_chain = if let Some(h) = rt.longest_chain() {
			h
		} else {
			return Ok(());
		};

		info!("committing chain {} to DHT", hex::encode(longest_chain));

		// Write the message with hash has to the blockchain, as well as its predecessor
		fn upload_chain_from(
			this: &mut Context,
			rt: &Rt,
			curr: &Hash,
			kad: &mut Kademlia<MemoryStore>,
		) -> Result<(), Error> {
			let msg = rt.get_message(&curr).ok_or(Error::MissingMessage)?;
			let msg_bytes = serde_json::to_vec(&msg).map_err(|e| Error::SerializationError(e))?;

			// Write the transaction under its hash with its JSON serialization to the DHT
			debug!(
				"writing message {} to KAD DHT",
				hex::encode(msg.hash().as_ref())
			);
			let q_id = kad.put_record(
				Record::new(RecordKey::new(&msg.hash().as_ref()), msg_bytes),
				DHT_QUORUM,
			)?;
			this.chain_uploads.insert(q_id, msg.hash().clone());

			if let Some(prev) = msg.data().prev() {
				upload_chain_from(this, rt, prev, kad)
			} else {
				Ok(())
			}
		}

		upload_chain_from(self, rt, longest_chain, kad)
	}

	/// Initiates a chain synchronization round. Results are reported through the LongestChainUpdated event.
	pub fn download_head(
		&mut self,
		request_response: &mut RRBehavior<Request, Response>,
		sampling_pool: Vec<&PeerId>,
	) -> Result<(), Error> {
		// Contact x% of peers
		let n_peers = sampling_pool.len();
		let to_contact = sampling_pool
			.into_iter()
			.take((n_peers as f32 * SAMPLING_SIZE) as usize);

		// Take note of which round of questioning this is
		let entry = SynchronizationRequest {
			initiated_at: Instant::now(),
			peers_contacted: to_contact
				.clone()
				.map(Clone::clone)
				.collect::<Vec<PeerId>>(),
			results: Vec::new(),
		};
		let entry_id = self.chain_downloads.len();
		self.chain_downloads.push(entry);

		// Request the longest chain from each peer
		for peer in to_contact {
			request_response.send_request(
				peer,
				Request::LongestChain {
					query_round: entry_id,
				},
			);
		}

		Ok(())
	}

	/// Initiates a download of the chain with HEAD head.
	pub fn download_msg(
		&mut self,
		head: &Hash,
		kad: &mut Kademlia<MemoryStore>,
	) -> Result<(), Error> {
		let q_id = kad.get_record(RecordKey::new(&head.as_ref()));
		self.message_downloads.insert(q_id);

		Ok(())
	}
}
