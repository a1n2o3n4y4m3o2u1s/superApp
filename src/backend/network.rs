use libp2p::{
    gossipsub, request_response, kad,
    swarm::{NetworkBehaviour, Swarm},
    PeerId,
};
#[cfg(not(target_arch = "wasm32"))]
use libp2p::mdns;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockRequest {
    Fetch(String),
    Store(Vec<u8>),
    LocalSearch(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockResponse {
    Block(Vec<u8>),
    SearchResults(Vec<Vec<u8>>),
    Ack,
    NotFound,
    Error(String),
}

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "MyBehaviourEvent")]
pub struct MyBehaviour {
    #[cfg(not(target_arch = "wasm32"))]
    pub mdns: mdns::tokio::Behaviour,
    pub gossipsub: gossipsub::Behaviour,
    pub request_response: request_response::cbor::Behaviour<BlockRequest, BlockResponse>,
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
}

#[derive(Debug)]
pub enum MyBehaviourEvent {
    #[cfg(not(target_arch = "wasm32"))]
    Mdns(mdns::Event),
    Gossipsub(gossipsub::Event),
    RequestResponse(request_response::Event<BlockRequest, BlockResponse>),
    Kad(kad::Event),
}

#[cfg(not(target_arch = "wasm32"))]
impl From<mdns::Event> for MyBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        MyBehaviourEvent::Mdns(event)
    }
}

impl From<gossipsub::Event> for MyBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        MyBehaviourEvent::Gossipsub(event)
    }
}

impl From<request_response::Event<BlockRequest, BlockResponse>> for MyBehaviourEvent {
    fn from(event: request_response::Event<BlockRequest, BlockResponse>) -> Self {
        MyBehaviourEvent::RequestResponse(event)
    }
}

impl From<kad::Event> for MyBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        MyBehaviourEvent::Kad(event)
    }
}

pub fn create_swarm(keypair: libp2p::identity::Keypair) -> Result<Swarm<MyBehaviour>, Box<dyn Error>> {
    #[cfg(not(target_arch = "wasm32"))]
    let builder = libp2p::SwarmBuilder::with_existing_identity(keypair)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default(),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?
        .with_dns()?;
    
    #[cfg(target_arch = "wasm32")]
    return Err("WASM networking not fully implemented. Please use Desktop for now.".into());

    #[cfg(not(target_arch = "wasm32"))]
    let swarm = builder.with_behaviour(|key| {
            // mDNS
            let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), PeerId::from(key.public()))
                .expect("Failed to create mDNS behaviour");

            // Gossipsub
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .message_id_fn(message_id_fn)
                .build()
                .map_err(|msg| std::io::Error::new(std::io::ErrorKind::Other, msg))
                .expect("Failed to build gossipsub config");

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )
            .expect("Failed to create gossipsub behaviour");

            // Request-Response
            let request_response = request_response::cbor::Behaviour::new(
                [(
                    libp2p::StreamProtocol::new("/superapp/sync/1.0.0"),
                    request_response::ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );

            // Kademlia
            let store = kad::store::MemoryStore::new(PeerId::from(key.public()));
            let kad_config = kad::Config::default();
            let kad = kad::Behaviour::with_config(PeerId::from(key.public()), store, kad_config);

            MyBehaviour {
                mdns,
                gossipsub,
                request_response,
                kad,
            }
        })?
        .build();

    #[cfg(not(target_arch = "wasm32"))]
    Ok(swarm)
}
