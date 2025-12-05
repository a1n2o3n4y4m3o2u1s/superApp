pub mod network;
pub mod dag;
pub mod store;
pub mod identity;
pub mod wasm;
pub mod vm;
use vm::VM;

use libp2p::{
    futures::StreamExt,
    gossipsub,
    identity::Keypair,
    request_response::{self, OutboundRequestId, ResponseChannel},
    swarm::SwarmEvent,
    PeerId,
    kad,
};
#[cfg(not(target_arch = "wasm32"))]
use libp2p::mdns;
use network::{BlockRequest, BlockResponse, MyBehaviour, MyBehaviourEvent};
use std::collections::HashMap;
use std::path::Path;
use store::Store;
use aes_gcm::{
    aead::{Aead, KeyInit, AeadCore},
    Aes256Gcm, Nonce,
};
use rand::rngs::OsRng;
use tokio::sync::mpsc;
use std::time::Duration;

#[derive(Debug)]
pub enum AppCmd {
    Init,
    PublishBlock(dag::DagNode),
    PublishProfile { name: String, bio: String },
    Vouch { target_peer_id: String },
    PublishPost { content: String, attachments: Vec<String>, geohash: Option<String> },
    PublishBlob { mime_type: String, data: String },
    FetchPosts,
    FetchLocalPosts { geohash_prefix: String },
    SendMessage { recipient: String, content: String },
    FetchMessages { peer_id: String },
    FetchMyProfile,
    MintToken { amount: u64 },
    SendToken { recipient: String, amount: u64 },
    ClaimToken { burn_cid: String },
    FetchPendingTransfers,
    FetchBalance,
    AutoDetectGeohash,
    ClaimUbi,
    FetchUbiTimer,
    CheckVerificationStatus,
    FetchUserProfile { peer_id: String },
    PublishWebPage { url: String, title: String, content: String, description: String, tags: Vec<String> },
    FetchWebPage { url: String },
    RegisterName { name: String, target: String },
    ResolveName { name: String },
    FetchBlock { cid: String, peer_id: Option<String> },
    FetchStorageStats,
    CreateListing { title: String, description: String, price: u64, image_cid: Option<String> },
    FetchListings,

    SearchWeb { query: String },
    DeployContract { code: String, init_params: String },
    CallContract { contract_id: String, method: String, params: String },
    FetchContracts,
    FetchContractState { contract_id: String },
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    PeerDiscovered(String),
    PeerConnected(String),
    BlockReceived(dag::DagNode),
    BlockFetched { cid: String, node: Option<dag::DagNode> },
    HistoryFetched(Vec<dag::DagNode>),
    MessageReceived(dag::DagNode, String), // Node + Decrypted Content
    MessagesFetched(Vec<(dag::DagNode, String)>), // List of (Node, Decrypted Content)
    MyIdentity(String),
    ProfileFetched(Option<dag::ProfilePayload>),
    BalanceFetched(i64),
    PendingTransfersFetched(Vec<dag::DagNode>),
    GeohashDetected(String),
    UbiTimerFetched(Option<u64>),
    VerificationStatus(VerificationStatus),
    UserProfileFetched(Option<dag::ProfilePayload>),
    WebPageFetched { url: String, content: Option<String> },
    Listening(String),
    NameResolved { name: String, target: Option<String> },
    StorageStatsFetched { block_count: usize, total_bytes: usize },
    LocalPostsFetched(Vec<dag::DagNode>),
    ListingsFetched(Vec<dag::DagNode>),
    WebSearchResults(Vec<dag::DagNode>),
    ContractsFetched(Vec<dag::DagNode>),
    ContractStateFetched { contract_id: String, state: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VerificationStatus {
    Unverified,
    Verified,
    Founder,
    EligibleForFounder,
}

pub struct Backend {
    swarm: libp2p::Swarm<MyBehaviour>,
    store: Store,
    cmd_rx: mpsc::UnboundedReceiver<AppCmd>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    pending_requests: HashMap<OutboundRequestId, String>, // RequestId -> CID
    keypair: Keypair,
    encryption_keypair: x25519_dalek::StaticSecret,
    pending_replications: HashMap<String, (dag::DagNode, std::time::Instant)>,
}

impl Backend {
    pub async fn new(
        store: Store,
        cmd_rx: mpsc::UnboundedReceiver<AppCmd>,
        event_tx: mpsc::UnboundedSender<AppEvent>,
        keypair_opt: Option<Keypair>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Load or generate identity
        let identity_path = Path::new("identity.pem");
        let keypair = if let Some(kp) = keypair_opt {
            kp
        } else {
            match identity::load_identity(identity_path) {
                Ok(kp) => {
                    println!("Loaded existing identity");
                    kp
                }
                Err(_) => {
                    println!("Generating new identity");
                    let kp = Keypair::generate_ed25519();
                    identity::save_identity(identity_path, &kp)?;
                    kp
                }
            }
        };

        // Load or generate encryption key
        let enc_key_path = Path::new("encryption.key");
        let encryption_keypair = if enc_key_path.exists() {
             let bytes = std::fs::read(enc_key_path)?;
             let arr: [u8; 32] = bytes.try_into().map_err(|_| "Invalid key length")?;
             x25519_dalek::StaticSecret::from(arr)
        } else {
             let key = x25519_dalek::StaticSecret::random_from_rng(OsRng);
             std::fs::write(enc_key_path, key.to_bytes())?;
             key
        };

        let mut swarm = network::create_swarm(keypair.clone())?;

        // Start listening
        #[cfg(not(target_arch = "wasm32"))]
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        // Subscribe to gossipsub topic
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&gossipsub::IdentTopic::new("blocks"))?;

        Ok(Self {
            swarm,
            store,
            cmd_rx,
            event_tx,
            pending_requests: HashMap::new(),
            keypair,
            encryption_keypair,
            pending_replications: HashMap::new(),
        })
    }

    pub async fn run(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        let mut replication_interval = tokio::time::interval(Duration::from_secs(5));
        
        #[cfg(target_arch = "wasm32")]
        let mut replication_interval = gloo_timers::future::IntervalStream::new(5000);

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    self.handle_swarm_event(event).await;
                }
                Some(cmd) = self.cmd_rx.recv() => {
                    self.handle_command(cmd).await;
                }
                _ = async {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        replication_interval.tick().await;
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        replication_interval.next().await;
                    }
                } => {
                    self.check_pending_replications();
                }
            }
        }
    }

    pub fn dial(&mut self, addr: libp2p::Multiaddr) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.dial(addr)?;
        Ok(())
    }

    pub fn local_peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    pub fn get_first_listener(&self) -> Option<libp2p::Multiaddr> {
        self.swarm.listeners().next().cloned()
    }

    fn is_verified(&self, peer_id: &str, visited: &mut std::collections::HashSet<String>) -> bool {
        if visited.contains(peer_id) {
            return false;
        }
        visited.insert(peer_id.to_string());

        // 1. Check if Founder
        if let Ok(Some(profile)) = self.store.get_profile(peer_id) {
            if let Some(fid) = profile.founder_id {
                if fid <= 100 {
                    return true;
                }
            }
        }

        // 2. Check proofs
        // We need to check if ANY of the proofs are signed by a verified user.
        match self.store.get_proofs(peer_id) {
            Ok(proofs) => {
                for proof in proofs {
                    // Recursive check
                    if self.is_verified(&proof.author, visited) {
                        return true;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    fn is_caller_verified(&self) -> bool {
        let author_pubkey = self.keypair.public();
        let author_hex = hex::encode(author_pubkey.encode_protobuf());
        let mut visited = std::collections::HashSet::new();
        self.is_verified(&author_hex, &mut visited)
    }

    fn try_decrypt(&self, node: &dag::DagNode) -> String {
        if let dag::DagPayload::Message(msg) = &node.payload {
            // If I am the recipient
            let my_pubkey = self.keypair.public();
            let my_hex = hex::encode(my_pubkey.encode_protobuf());
            
            if msg.recipient == my_hex {
                // Decrypt
                if let Ok(ephemeral_pub_bytes) = hex::decode(&msg.ephemeral_pubkey) {
                    if ephemeral_pub_bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&ephemeral_pub_bytes);
                        let ephemeral_pk = x25519_dalek::PublicKey::from(arr);
                        let shared_secret = self.encryption_keypair.diffie_hellman(&ephemeral_pk);
                        
                        let shared_secret_bytes = shared_secret.as_bytes();
                        let key_array = aes_gcm::Key::<Aes256Gcm>::from_slice(shared_secret_bytes);
                        let key = Aes256Gcm::new(key_array);
                        
                        if let Ok(nonce_bytes) = hex::decode(&msg.nonce) {
                            if nonce_bytes.len() == 12 {
                                let nonce = aes_gcm::aead::generic_array::GenericArray::from_slice(&nonce_bytes);
                                if let Ok(ciphertext_bytes) = hex::decode(&msg.ciphertext) {
                                    if let Ok(plaintext) = key.decrypt(nonce, ciphertext_bytes.as_ref()) {
                                        return String::from_utf8_lossy(&plaintext).to_string();
                                    }
                                }
                            }
                        }
                    }
                }
                return "[Decryption Failed]".to_string();
            } else if node.author == my_hex {
                // I sent it. I can't decrypt it (ephemeral key lost), but I might have stored it?
                // For now, return placeholder.
                return "[Sent Message - Content Encrypted]".to_string();
            }
        }
        "[Not a message]".to_string()
    }

    fn replicate_block(&mut self, node: &dag::DagNode) {
        let connected_peers: Vec<PeerId> = self.swarm.connected_peers().cloned().collect();
        let connected_count = connected_peers.len();
        
        let target_replication_count = 10;
        let mut target_peers = connected_peers.clone();

        // If we don't have enough connected peers, try to find more via DHT
        if connected_count < target_replication_count {
            println!("Not enough connected peers ({}/{}), querying DHT...", connected_count, target_replication_count);
            self.swarm.behaviour_mut().kad.get_closest_peers(node.id.as_bytes().to_vec());
            
            // Add to pending replications to retry later
            if !self.pending_replications.contains_key(&node.id) {
                self.pending_replications.insert(node.id.clone(), (node.clone(), std::time::Instant::now()));
                println!("Added block {} to pending replication queue", node.id);
            }
        } else {
            // We have enough peers, remove from pending if it was there
            self.pending_replications.remove(&node.id);
        }

        // Send to available peers anyway
        let peers_to_send: Vec<PeerId> = target_peers.into_iter().take(target_replication_count).collect();
        
        if !peers_to_send.is_empty() {
             println!("Replicating block {} to {} peers", node.id, peers_to_send.len());
             
             let data = match serde_json::to_vec(node) {
                 Ok(d) => d,
                 Err(e) => {
                     eprintln!("Failed to serialize node for replication: {:?}", e);
                     return;
                 }
             };

             let request = BlockRequest::Store(data);
             
             for peer in peers_to_send {
                 self.swarm.behaviour_mut().request_response.send_request(&peer, request.clone());
             }
        }
    }

    fn check_pending_replications(&mut self) {
        let now = std::time::Instant::now();
        let timeout = Duration::from_secs(60); // Stop trying after 60 seconds
        
        let mut to_remove = Vec::new();
        let mut to_retry = Vec::new();

        for (id, (node, started)) in &self.pending_replications {
            if now.duration_since(*started) > timeout {
                to_remove.push(id.clone());
                println!("Replication timed out for block {}", id);
            } else {
                to_retry.push(node.clone());
            }
        }

        for id in to_remove {
            self.pending_replications.remove(&id);
        }

        for node in to_retry {
            // This will check connected peers again and potentially re-send and re-query DHT
            self.replicate_block(&node);
        }
    }

    async fn handle_command(&mut self, cmd: AppCmd) {
        match cmd {
            AppCmd::Init => {
                println!("Backend initialized");
            }
            AppCmd::PublishBlock(node) => {
                // 1. Store locally
                if let Err(e) = self.store.put_node(&node) {
                    eprintln!("Failed to store node: {:?}", e);
                    return;
                }

                // 2. Publish CID to gossipsub
                let topic = gossipsub::IdentTopic::new("blocks");
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                    eprintln!("Failed to publish block CID: {:?}", e);
                }
                
                // 3. Replicate
                self.replicate_block(&node);
            }
            AppCmd::PublishProfile { name, bio } => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                // Check if we already have a founder_id from previous profile
                let mut founder_id = None;
                
                // Get previous head for this user if any
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => {
                        // Check if previous head was a profile and had founder_id
                        if let Ok(Some(node)) = self.store.get_node(&cid) {
                             if let dag::DagPayload::Profile(p) = node.payload {
                                 founder_id = p.founder_id;
                             }
                        }
                        vec![cid]
                    },
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                // If no founder_id, check if we can claim it
                if founder_id.is_none() {
                    match self.store.count_unique_profiles() {
                        Ok(count) => {
                            if count < 100 {
                                founder_id = Some((count + 1) as u32);
                            }
                        }
                        Err(e) => eprintln!("Failed to count profiles: {:?}", e),
                    }
                }

                let encryption_pubkey = Some(hex::encode(x25519_dalek::PublicKey::from(&self.encryption_keypair).to_bytes()));

                let payload = dag::DagPayload::Profile(dag::ProfilePayload { name, bio, founder_id, encryption_pubkey });
                
                match dag::DagNode::new(
                    "profile:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created profile node: {}", node.id);
                        // 1. Store locally
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store profile node: {:?}", e);
                            return;
                        }
                        
                        // 2. Update head
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }

                        // 3. Publish CID to gossipsub
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish profile CID: {:?}", e);
                        }

                        // Emit event
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        
                        // 4. Replicate
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create profile node: {:?}", e),
                }
            }
            AppCmd::Vouch { target_peer_id } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot vouch: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Proof(dag::ProofPayload { target_pubkey: target_peer_id });
                
                // Get previous head for this user if any
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "proof:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created proof node: {}", node.id);
                        // 1. Store locally
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store proof node: {:?}", e);
                            return;
                        }
                        
                        // 2. Update head
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }

                        // 3. Publish CID to gossipsub
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish proof CID: {:?}", e);
                        }
                        
                        // 4. Replicate
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create proof node: {:?}", e),
                }
            }
            AppCmd::FetchPosts => {
                match self.store.get_recent_posts(50) {
                    Ok(posts) => {
                        let _ = self.event_tx.send(AppEvent::HistoryFetched(posts));
                    }
                    Err(e) => eprintln!("Failed to fetch posts: {:?}", e),
                }
            }
            AppCmd::FetchLocalPosts { geohash_prefix } => {
                match self.store.get_local_posts(&geohash_prefix, 50) {
                    Ok(posts) => {
                        let _ = self.event_tx.send(AppEvent::LocalPostsFetched(posts));
                    }
                    Err(e) => eprintln!("Failed to fetch local posts: {:?}", e),
                }
            }
            AppCmd::PublishPost { content, attachments, geohash } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish post: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Post(dag::PostPayload { content, attachments, geohash });
                
                // Get previous head for this user if any
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "post:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created post node: {}", node.id);
                        // 1. Store locally
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store post node: {:?}", e);
                            return;
                        }
                        
                        // 2. Update head
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }

                        // 3. Publish CID to gossipsub
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish post CID: {:?}", e);
                        }
                        
                        // 4. Notify frontend so it can display immediately
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        
                        // 5. Replicate
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create post node: {:?}", e),
                }
            }
            AppCmd::PublishBlob { mime_type, data } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish blob: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Blob(dag::BlobPayload { mime_type, data });
                
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "blob:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created blob node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store blob node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish blob CID: {:?}", e);
                        }
                        
                        // Notify frontend so it knows the CID (we might need to send a specific event or generic BlockReceived)
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create blob node: {:?}", e),
                }
            }
            AppCmd::FetchMessages { peer_id } => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                match self.store.get_messages(&author_hex, &peer_id) {
                    Ok(messages) => {
                        let decrypted_messages = messages.into_iter().map(|node| {
                            let content = self.try_decrypt(&node);
                            (node, content)
                        }).collect();
                        let _ = self.event_tx.send(AppEvent::MessagesFetched(decrypted_messages));
                    }
                    Err(e) => eprintln!("Failed to fetch messages: {:?}", e),
                }
            }
            AppCmd::SendMessage { recipient, content } => {
                // Note: We might want to allow unverified users to message verified users (e.g. to ask for verification)?
                // The plan says "No app access whatsoever until verified".
                // So strict blocking for now.
                if !self.is_caller_verified() {
                    eprintln!("Cannot send message: User is not verified.");
                    return;
                }
                // 1. Fetch recipient's profile to get their public key
                let recipient_pubkey_bytes = match self.store.get_profile(&recipient) {
                    Ok(Some(profile)) => {
                        if let Some(pk_hex) = profile.encryption_pubkey {
                            match hex::decode(pk_hex) {
                                Ok(bytes) => {
                                    if bytes.len() == 32 {
                                        let mut arr = [0u8; 32];
                                        arr.copy_from_slice(&bytes);
                                        Some(x25519_dalek::PublicKey::from(arr))
                                    } else {
                                        None
                                    }
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(recipient_pk) = recipient_pubkey_bytes {
                    // 2. Generate ephemeral keypair
                    let ephemeral_secret = x25519_dalek::StaticSecret::random_from_rng(OsRng);
                    let ephemeral_public = x25519_dalek::PublicKey::from(&ephemeral_secret);

                    // 3. Perform ECDH
                    let shared_secret = ephemeral_secret.diffie_hellman(&recipient_pk);

                    // 4. Encrypt
                    let key = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(shared_secret.as_bytes()));
                    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
                    
                    match key.encrypt(&nonce, content.as_bytes()) {
                        Ok(ciphertext_bytes) => {
                            let payload = dag::DagPayload::Message(dag::MessagePayload {
                                recipient: recipient.clone(),
                                ciphertext: hex::encode(ciphertext_bytes),
                                nonce: hex::encode(nonce),
                                ephemeral_pubkey: hex::encode(ephemeral_public.to_bytes()),
                            });

                            // Get previous head for this user if any
                            let author_pubkey = self.keypair.public();
                            let author_hex = hex::encode(author_pubkey.encode_protobuf());
                            
                            let prev = match self.store.get_head(&author_hex) {
                                Ok(Some(cid)) => vec![cid],
                                Ok(None) => vec![],
                                Err(e) => {
                                    eprintln!("Failed to get head: {:?}", e);
                                    vec![]
                                }
                            };

                            match dag::DagNode::new(
                                "message:v1".to_string(),
                                payload,
                                prev,
                                &self.keypair,
                                0
                            ) {
                                Ok(node) => {
                                    println!("Created encrypted message node: {}", node.id);
                                    // 1. Store locally
                                    if let Err(e) = self.store.put_node(&node) {
                                        eprintln!("Failed to store message node: {:?}", e);
                                        return;
                                    }
                                    
                                    // 2. Update head
                                    if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                                        eprintln!("Failed to update head: {:?}", e);
                                    }

                                    // 3. Publish CID to gossipsub
                                    let topic = gossipsub::IdentTopic::new("blocks");
                                    if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                                        eprintln!("Failed to publish message CID: {:?}", e);
                                    }
                                    
                                    // 4. Notify frontend so it can display immediately
                                    // We pass the plaintext content here because we just created it!
                                    let _ = self.event_tx.send(AppEvent::MessageReceived(node.clone(), content));
                                    
                                    // 5. Replicate
                                    self.replicate_block(&node);
                                }
                                Err(e) => eprintln!("Failed to create message node: {:?}", e),
                            }
                        }
                        Err(e) => eprintln!("Encryption failed: {:?}", e),
                    }
                } else {
                    eprintln!("Cannot send message: Recipient has no encryption key");
                }
            }

            AppCmd::FetchMyProfile => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => {
                        match self.store.get_node(&cid) {
                            Ok(Some(node)) => {
                                if let dag::DagPayload::Profile(profile) = node.payload {
                                    let _ = self.event_tx.send(AppEvent::ProfileFetched(Some(profile)));
                                } else {
                                    // Head is not a profile (maybe a post), we need to traverse back to find the last profile?
                                    // For now, assuming head is profile if user just updated it. 
                                    // Ideally we should search for the latest profile node.
                                    // But the plan said "Get head... Extract ProfilePayload". 
                                    // Let's stick to that for now, or maybe just return None if head is not profile.
                                    // Actually, if head is a post, we should probably look at the chain.
                                    // But for simplicity in this iteration, let's just return None if head is not profile.
                                    let _ = self.event_tx.send(AppEvent::ProfileFetched(None));
                                }
                            }
                            Ok(None) => {
                                let _ = self.event_tx.send(AppEvent::ProfileFetched(None));
                            }
                            Err(e) => eprintln!("Failed to get head node: {:?}", e),
                        }
                    }
                    Ok(None) => {
                        let _ = self.event_tx.send(AppEvent::ProfileFetched(None));
                    }
                    Err(e) => eprintln!("Failed to get head: {:?}", e),
                }
            }

            AppCmd::CheckVerificationStatus => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let mut status = VerificationStatus::Unverified;

                // 1. Check if Founder
                // We need to fetch our own profile to see if we have a founder_id
                if let Ok(Some(profile)) = self.store.get_profile(&author_hex) {
                    if let Some(fid) = profile.founder_id {
                        if fid <= 100 {
                            status = VerificationStatus::Founder;
                        }
                    }
                }

                // 2. If not founder, check proofs (recursively)
                if status == VerificationStatus::Unverified {
                    if self.is_caller_verified() {
                        status = VerificationStatus::Verified;
                    } else {
                        // 3. Check if eligible for founder (first 100)
                        match self.store.count_unique_profiles() {
                            Ok(count) => {
                                if count < 100 {
                                    status = VerificationStatus::EligibleForFounder;
                                }
                            }
                            Err(e) => eprintln!("Failed to count profiles: {:?}", e),
                        }
                    }
                }

                let _ = self.event_tx.send(AppEvent::VerificationStatus(status));
            }

            AppCmd::FetchUserProfile { peer_id } => {
                match self.store.get_profile(&peer_id) {
                    Ok(profile) => {
                        let _ = self.event_tx.send(AppEvent::UserProfileFetched(profile));
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch user profile: {:?}", e);
                        let _ = self.event_tx.send(AppEvent::UserProfileFetched(None));
                    }
                }
            }


            AppCmd::MintToken { amount } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot mint token: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Token(dag::TokenPayload {
                    action: dag::TokenAction::Mint,
                    amount,
                    target: None,
                    memo: Some("UBI Mint".to_string()),
                    ref_cid: None,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "token:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created mint node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store mint node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish mint CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        
                        // Replicate
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create mint node: {:?}", e),
                }
            }

            AppCmd::SendToken { recipient, amount } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot send token: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Token(dag::TokenPayload {
                    action: dag::TokenAction::Burn,
                    amount,
                    target: Some(recipient),
                    memo: Some("Transfer".to_string()),
                    ref_cid: None,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "token:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created transfer node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store transfer node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish transfer CID: {:?}", e);
                        }
                         let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                         
                         // Replicate
                         self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create transfer node: {:?}", e),
                }
            }

            AppCmd::ClaimToken { burn_cid } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot claim token: User is not verified.");
                    return;
                }
                // 1. Fetch the burn block to verify and get amount
                match self.store.get_node(&burn_cid) {
                    Ok(Some(burn_node)) => {
                        if let dag::DagPayload::Token(ref token) = burn_node.payload {
                             if token.action == dag::TokenAction::Burn {
                                 // Check if we are the target
                                 let author_pubkey = self.keypair.public();
                                 let author_hex = hex::encode(author_pubkey.encode_protobuf());
                                 
                                 if let Some(target) = &token.target {
                                     if target == &author_hex {
                                         // Create Claim Node
                                         let payload = dag::DagPayload::Token(dag::TokenPayload {
                                             action: dag::TokenAction::TransferClaim,
                                             amount: token.amount,
                                             target: None,
                                             memo: Some("Claim Transfer".to_string()),
                                             ref_cid: Some(burn_cid.clone()),
                                         });

                                         let prev = match self.store.get_head(&author_hex) {
                                             Ok(Some(cid)) => vec![cid],
                                             Ok(None) => vec![],
                                             Err(e) => {
                                                 eprintln!("Failed to get head: {:?}", e);
                                                 vec![]
                                             }
                                         };

                                         match dag::DagNode::new(
                                             "token:v1".to_string(),
                                             payload,
                                             prev,
                                             &self.keypair,
                                             0
                                         ) {
                                             Ok(node) => {
                                                 println!("Created claim node: {}", node.id);
                                                 if let Err(e) = self.store.put_node(&node) {
                                                     eprintln!("Failed to store claim node: {:?}", e);
                                                     return;
                                                 }
                                                 if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                                                     eprintln!("Failed to update head: {:?}", e);
                                                 }
                                                 let topic = gossipsub::IdentTopic::new("blocks");
                                                 if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                                                     eprintln!("Failed to publish claim CID: {:?}", e);
                                                 }
                                                 let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                                                 
                                                 // Replicate
                                                 self.replicate_block(&node);
                                                 
                                                 // Refresh balance and pending transfers
                                                 let _ = self.event_tx.send(AppEvent::BalanceFetched(self.store.get_balance(&author_hex).unwrap_or(0)));
                                                 // We should also re-fetch pending transfers but that requires sending another event or command.
                                                 // For now, the frontend can trigger it.
                                             }
                                             Err(e) => eprintln!("Failed to create claim node: {:?}", e),
                                         }
                                     } else {
                                         eprintln!("Claim failed: Target mismatch");
                                     }
                                 } else {
                                     eprintln!("Claim failed: Not a burn block");
                                 }
                             } else {
                                 eprintln!("Claim failed: Not a token block");
                             }
                        } else {
                            eprintln!("Claim failed: Not a token payload");
                        }
                    }
                    Ok(None) => eprintln!("Claim failed: Burn block not found"),
                    Err(e) => eprintln!("Claim failed: Store error {:?}", e),
                }
            }

            AppCmd::FetchPendingTransfers => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                match self.store.get_pending_transfers(&author_hex) {
                    Ok(pending) => {
                        let _ = self.event_tx.send(AppEvent::PendingTransfersFetched(pending));
                    }
                    Err(e) => eprintln!("Failed to fetch pending transfers: {:?}", e),
                }
            }

            AppCmd::FetchBalance => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                match self.store.get_balance(&author_hex) {
                    Ok(balance) => {
                        let _ = self.event_tx.send(AppEvent::BalanceFetched(balance));
                    }
                    Err(e) => eprintln!("Failed to fetch balance: {:?}", e),
                }
            }

            AppCmd::AutoDetectGeohash => {
                println!("Auto-detecting geohash...");
                let event_tx = self.event_tx.clone();
                
                #[cfg(not(target_arch = "wasm32"))]
                tokio::spawn(async move {
                    detect_geohash(event_tx).await;
                });
                
                #[cfg(target_arch = "wasm32")]
                wasm_bindgen_futures::spawn_local(async move {
                    detect_geohash(event_tx).await;
                });
            }

            AppCmd::FetchUbiTimer => {
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                match self.store.get_last_ubi_claim(&author_hex) {
                    Ok(timestamp) => {
                        let _ = self.event_tx.send(AppEvent::UbiTimerFetched(timestamp));
                    }
                    Err(e) => eprintln!("Failed to fetch UBI timer: {:?}", e),
                }
            }

            AppCmd::ClaimUbi => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot claim UBI: User is not verified.");
                    return;
                }
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                // Check last claim time
                let can_claim = match self.store.get_last_ubi_claim(&author_hex) {
                    Ok(Some(last_ts)) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        // 24 hours = 86400 seconds
                        if now > last_ts + 86400 {
                            true
                        } else {
                            println!("Cannot claim UBI yet. Next claim in {} seconds", (last_ts + 86400) - now);
                            false
                        }
                    }
                    Ok(None) => true, // First time claiming
                    Err(e) => {
                        eprintln!("Failed to check UBI timer: {:?}", e);
                        false
                    }
                };

                if can_claim {
                    let payload = dag::DagPayload::Token(dag::TokenPayload {
                        action: dag::TokenAction::Mint,
                        amount: 10,
                        target: None,
                        memo: Some("UBI Daily Claim".to_string()),
                        ref_cid: None,
                    });
                    
                    let prev = match self.store.get_head(&author_hex) {
                        Ok(Some(cid)) => vec![cid],
                        Ok(None) => vec![],
                        Err(e) => {
                            eprintln!("Failed to get head: {:?}", e);
                            vec![]
                        }
                    };

                    match dag::DagNode::new(
                        "token:v1".to_string(),
                        payload,
                        prev,
                        &self.keypair,
                        0
                    ) {
                        Ok(node) => {
                            println!("Created UBI mint node: {}", node.id);
                            if let Err(e) = self.store.put_node(&node) {
                                eprintln!("Failed to store UBI mint node: {:?}", e);
                                return;
                            }
                            if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                                eprintln!("Failed to update head: {:?}", e);
                            }
                            let topic = gossipsub::IdentTopic::new("blocks");
                            if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                                eprintln!("Failed to publish UBI mint CID: {:?}", e);
                            }
                            let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                            
                            // Replicate
                            self.replicate_block(&node);
                            
                            // Refresh balance and timer
                            let _ = self.event_tx.send(AppEvent::BalanceFetched(self.store.get_balance(&author_hex).unwrap_or(0)));
                            let _ = self.event_tx.send(AppEvent::UbiTimerFetched(Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs())));
                        }
                        Err(e) => eprintln!("Failed to create UBI mint node: {:?}", e),
                    }
                }
            }

            AppCmd::PublishWebPage { url, title, content, description, tags } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish web page: User is not verified.");
                    return;
                }
                
                // If content is hex-encoded WASM or starts with magic bytes, it stays as is
                // No special processing needed here as VM::render_web_page handles detection,
                // but we might want to validate size etc.

                let payload = dag::DagPayload::Web(dag::WebPayload {
                    url: url.clone(),
                    title,
                    content,
                    description,
                    tags: tags.clone(),
                });

                let author_hex = hex::encode(self.keypair.public().encode_protobuf());
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };

                match dag::DagNode::new("web:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                         println!("Published web page: {}", node.id);
                         if let Err(e) = self.store.put_node(&node) {
                             eprintln!("Failed to store web node: {:?}", e);
                         } else {
                              // Announce tags to DHT
                              for tag in tags {
                                  let key = kad::RecordKey::new(&format!("search:term:{}", tag).into_bytes());
                                  println!("Announcing provider for tag: {}", tag);
                                  self.swarm.behaviour_mut().kad.start_providing(key).ok();
                              }
                         }
                         
                         if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                              eprintln!("Failed to update head: {:?}", e);
                         }

                         let topic = gossipsub::IdentTopic::new("blocks");
                         let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());

                        let _ = self.event_tx.send(AppEvent::WebPageFetched { url, content: Some("<h1>Published!</h1>".to_string()) });
                    }
                     Err(e) => eprintln!("Failed to create web node: {:?}", e),
                }
            }

            AppCmd::FetchListings => {
                match self.store.get_active_listings(50) {
                    Ok(listings) => {
                        let _ = self.event_tx.send(AppEvent::ListingsFetched(listings));
                    }
                    Err(e) => eprintln!("Failed to fetch listings: {:?}", e),
                }
            }

            AppCmd::SearchWeb { query } => {
                 // 1. Local Search
                 match self.store.search_web_pages(&query) {
                     Ok(nodes) => {
                         let _ = self.event_tx.send(AppEvent::WebSearchResults(nodes));
                     },
                     Err(e) => eprintln!("Local search failed: {:?}", e),
                 }

                 // 2. DHT Search (Distributed)
                 // We treat the query as a tag for now
                 println!("Starting DHT search for: {}", query);
                 let key = kad::RecordKey::new(&format!("search:term:{}", query).into_bytes());
                 self.swarm.behaviour_mut().kad.get_providers(key);
            }
            AppCmd::DeployContract { code, init_params } => {
                 if !self.is_caller_verified() {
                    eprintln!("Cannot deploy contract: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Contract(dag::ContractPayload { code, init_params });
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                 let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "contract:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created contract node: {}", node.id);
                         if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store contract node: {:?}", e);
                            return;
                        }
                         if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish contract CID: {:?}", e);
                        }
                        self.replicate_block(&node);
                        
                        // Treat as block received to update UI if needed
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                    }
                    Err(e) => eprintln!("Failed to create contract node: {:?}", e),
                }
            }
            AppCmd::CallContract { contract_id, method, params } => {
                 if !self.is_caller_verified() {
                    eprintln!("Cannot call contract: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::ContractCall(dag::ContractCallPayload { contract_id, method, params });
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                 let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                 match dag::DagNode::new(
                    "contract_call:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created contract call node: {}", node.id);
                         if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store contract call node: {:?}", e);
                            return;
                        }
                         if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish contract call CID: {:?}", e);
                        }
                        self.replicate_block(&node);
                        
                        // Treat as block received
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                    }
                    Err(e) => eprintln!("Failed to create contract call node: {:?}", e),
                }
            }
            AppCmd::FetchContracts => {
                match self.store.get_contracts() {
                    Ok(contracts) => {
                        let _ = self.event_tx.send(AppEvent::ContractsFetched(contracts));
                    }
                    Err(e) => eprintln!("Failed to fetch contracts: {:?}", e),
                }
            }

            AppCmd::FetchContractState { contract_id } => {
                // 1. Get the contract itself to find initial state
                let (init_state, code) = match self.store.get_node(&contract_id) {
                    Ok(Some(node)) => {
                        if let dag::DagPayload::Contract(c) = node.payload {
                            (c.init_params, c.code)
                        } else {
                            ("{}".to_string(), "".to_string())
                        }
                    },
                    _ => ("{}".to_string(), "".to_string()),
                };

                // 2. Get all calls
                let calls = match self.store.get_contract_calls(&contract_id) {
                     Ok(calls) => calls,
                     Err(_) => vec![],
                };

                // 3. Calculate State via VM
                let final_state_str = VM::calculate_contract_state(&init_state, &code, &calls);

                let _ = self.event_tx.send(AppEvent::ContractStateFetched { contract_id, state: final_state_str });
            }


            AppCmd::FetchWebPage { url } => {
                let process_content = |content: String| -> String {
                     VM::render_web_page(&content)
                };

                // 1. Try to get web page directly
                match self.store.get_web_page(&url) {
                    Ok(Some(content)) => {
                        let final_content = process_content(content);
                        let _ = self.event_tx.send(AppEvent::WebPageFetched { url: url.clone(), content: Some(final_content) });
                    }
                    Ok(None) => {
                        // 2. Try to resolve as name
                        match self.store.get_name_record(&url) {
                            Ok(Some(target)) => {
                                // Found a name record, try to fetch the target
                                println!("Resolved {} to {}", url, target);
                                match self.store.get_web_page(&target) {
                                    Ok(Some(content)) => {
                                        let final_content = process_content(content);
                                        let _ = self.event_tx.send(AppEvent::WebPageFetched { url: url.clone(), content: Some(final_content) });
                                    }
                                    Ok(None) => {
                                        // Not found locally, try DHT for target
                                        println!("Content for {} not found locally, querying DHT...", target);
                                        let key = kad::RecordKey::new(&target.as_bytes());
                                        self.swarm.behaviour_mut().kad.get_providers(key);
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to fetch target page: {:?}", e);
                                        let _ = self.event_tx.send(AppEvent::WebPageFetched { url: url.clone(), content: None });
                                    }
                                }
                            }
                            Ok(None) => {
                                // Not found as page or name locally.
                                // Try DHT for the URL itself
                                println!("{} not found locally, querying DHT...", url);
                                let key = kad::RecordKey::new(&url.as_bytes());
                                self.swarm.behaviour_mut().kad.get_providers(key);
                            }
                            Err(e) => {
                                eprintln!("Failed to resolve name: {:?}", e);
                                let _ = self.event_tx.send(AppEvent::WebPageFetched { url: url.clone(), content: None });
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch web page: {:?}", e);
                        let _ = self.event_tx.send(AppEvent::WebPageFetched { url: url.clone(), content: None });
                    }
                }
            }

            AppCmd::RegisterName { name, target } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot register name: User is not verified.");
                    return;
                }
                
                let payload = dag::DagPayload::Name(dag::NamePayload { name: name.clone(), target });
                
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "name:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created name registration node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store name node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish name CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create name node: {:?}", e),
                }
            }

            AppCmd::ResolveName { name } => {
                match self.store.get_name_record(&name) {
                    Ok(target) => {
                         let _ = self.event_tx.send(AppEvent::NameResolved { name, target });
                    }
                    Err(e) => {
                         eprintln!("Failed to resolve name: {:?}", e);
                         let _ = self.event_tx.send(AppEvent::NameResolved { name, target: None });
                    }
                }
            }

            AppCmd::FetchBlock { cid, peer_id } => {
                // 1. Check local store
                if let Ok(Some(node)) = self.store.get_node(&cid) {
                     let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: Some(node) });
                     return;
                }
                
                // 2. Fetch from peer or DHT
                let do_dht = if let Some(pid_str) = peer_id {
                    if pid_str.is_empty() {
                        true
                    } else {
                        if let Ok(peer) = pid_str.parse::<PeerId>() {
                            let request_id = self.swarm.behaviour_mut().request_response.send_request(&peer, BlockRequest::Fetch(cid.clone()));
                            self.pending_requests.insert(request_id, cid.clone());
                            false
                        } else {
                            eprintln!("Invalid peer id: {}", pid_str);
                            true
                        }
                    }
                } else {
                    true
                };

                if do_dht {
                     // Query DHT
                     println!("Querying DHT for block {}", cid);
                     let key = kad::RecordKey::new(&cid.as_bytes());
                     self.swarm.behaviour_mut().kad.get_providers(key);
                }
            }

            AppCmd::FetchStorageStats => {
                match self.store.get_storage_stats() {
                    Ok((block_count, total_bytes)) => {
                        let _ = self.event_tx.send(AppEvent::StorageStatsFetched { block_count, total_bytes });
                    }
                    Err(e) => eprintln!("Failed to get storage stats: {:?}", e),
                }
            }

            AppCmd::CreateListing { title, description, price, image_cid } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot create listing: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Listing(dag::ListingPayload {
                    title,
                    description,
                    price,
                    image_cid,
                    status: dag::ListingStatus::Active,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = hex::encode(author_pubkey.encode_protobuf());
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "listing:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created listing node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store listing node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish listing CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create listing node: {:?}", e),
                }
            }
        }
    }


    async fn handle_swarm_event(&mut self, event: SwarmEvent<MyBehaviourEvent>) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {:?}", address);
                let _ = self.event_tx.send(AppEvent::Listening(address.to_string()));
            }
            #[cfg(not(target_arch = "wasm32"))]
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                for (peer_id, addr) in list {
                    println!("Discovered {:?}", peer_id);
                    let _ = self.event_tx.send(AppEvent::PeerDiscovered(peer_id.to_string()));
                    self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    self.swarm.behaviour_mut().kad.add_address(&peer_id, addr);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _addr) in list {
                    println!("Expired {:?}", peer_id);
                    self.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    self.swarm.behaviour_mut().kad.remove_peer(&peer_id);
                }
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::RequestResponse(event)) => {
                match event {
                    request_response::Event::Message { peer, message } => {
                        match message {
                            request_response::Message::Request { request, channel, .. } => {
                                match request {
                                    BlockRequest::Fetch(cid) => {
                                        println!("Received fetch request for block: {}", cid);
                                        let response_data = match self.store.get_block_bytes(&cid) {
                                            Ok(Some(bytes)) => bytes,
                                            _ => vec![],
                                        };

                                        let response = if response_data.is_empty() {
                                            BlockResponse::NotFound
                                        } else {
                                            BlockResponse::Block(response_data)
                                        };
                                        let _ = self.swarm.behaviour_mut().request_response.send_response(channel, response);
                                    }
                                    BlockRequest::LocalSearch(query) => {
                                        println!("Received local search request for: {}", query);
                                        let results = match self.store.search_web_pages(&query) {
                                             Ok(nodes) => {
                                                 nodes.iter().filter_map(|n| serde_json::to_vec(n).ok()).collect()
                                             },
                                             _ => vec![],
                                        };
                                        let response = BlockResponse::SearchResults(results);
                                        let _ = self.swarm.behaviour_mut().request_response.send_response(channel, response);
                                    }
                                    BlockRequest::Store(data) => {
                                        println!("Received store request");
                                        match serde_json::from_slice::<dag::DagNode>(&data) {
                                            Ok(node) => {
                                                // Verify and store
                                                match node.verify() {
                                                    Ok(true) => {
                                                        if let Err(e) = self.store.put_node(&node) {
                                                            eprintln!("Failed to store pushed node: {:?}", e);
                                                            let _ = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse::Error(format!("Store failed: {:?}", e)));
                                                        } else {
                                                            println!("Stored pushed node: {}", node.id);
                                                            // Also emit event so UI updates if relevant
                                                            if let dag::DagPayload::Message(_) = node.payload {
                                                                 let content = self.try_decrypt(&node);
                                                                 let _ = self.event_tx.send(AppEvent::MessageReceived(node.clone(), content));
                                                            } else {
                                                                 let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                                                            }
                                                            let _ = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse::Ack);
                                                        }
                                                    }
                                                    Ok(false) => {
                                                        let _ = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse::Error("Verification failed".to_string()));
                                                    }
                                                    Err(e) => {
                                                        let _ = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse::Error(e.to_string()));
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let _ = self.swarm.behaviour_mut().request_response.send_response(channel, BlockResponse::Error(e.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                            request_response::Message::Response { request_id, response } => {
                                if let Some(cid) = self.pending_requests.remove(&request_id) {
                                    match response {
                                        BlockResponse::Block(data) => {
                                            println!("Received response for block {}", cid);
                                            match serde_json::from_slice::<dag::DagNode>(&data) {
                                                Ok(node) => {
                                                    // Store and notify
                                                    let _ = self.store.put_node(&node);
                                                    
                                                    // If it's a web page, notify that it might be what we are looking for
                                                    if let dag::DagPayload::Web(web) = &node.payload {
                                                         let process_content = |content: String| -> String {
                                                            VM::render_web_page(&content)
                                                         };
                                                         let _ = self.event_tx.send(AppEvent::WebPageFetched { url: web.url.clone(), content: Some(process_content(web.content.clone())) });
                                                    }
                                                    // Also emit BlockFetched for the specific request
                                                    let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: Some(node) });
                                                }
                                                Err(e) => {
                                                    eprintln!("Failed to deserialize fetched node: {:?}", e);
                                                    let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: None });
                                                }
                                            }
                                        }
                                        BlockResponse::NotFound => {
                                            println!("Block {} not found on peer", cid);
                                            let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: None });
                                        }
                                        BlockResponse::Ack => {
                                            println!("Replication ACK received for {}", cid);
                                            // TODO: Increment replication count for this block in store/mem
                                        }
                                        BlockResponse::Error(e) => {
                                            eprintln!("Request error for {}: {}", cid, e);
                                            let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: None });
                                        }
                                        BlockResponse::SearchResults(results) => {
                                            println!("Received search results: {} items", results.len());
                                            let mut nodes = Vec::new();
                                            for bytes in results {
                                                if let Ok(node) = serde_json::from_slice::<dag::DagNode>(&bytes) {
                                                    nodes.push(node);
                                                }
                                            }
                                            if !nodes.is_empty() {
                                                let _ = self.event_tx.send(AppEvent::WebSearchResults(nodes));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    request_response::Event::OutboundFailure { request_id, error, .. } => {
                        eprintln!("Request failed: {:?}", error);
                        if let Some(cid) = self.pending_requests.remove(&request_id) {
                            let _ = self.event_tx.send(AppEvent::BlockFetched { cid, node: None });
                        }
                    }
                    _ => {}
                }
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                let _ = self.event_tx.send(AppEvent::PeerConnected(peer_id.to_string()));
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Kad(event)) => {
                match event {
                    kad::Event::OutboundQueryProgressed { result, .. } => {
                        match result {
                            kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. })) => {
                                let url = String::from_utf8(key.to_vec()).unwrap_or_default();
                                println!("Found providers for {}: {:?}", url, providers);
                                for peer in providers {
                                    let request_id = self.swarm.behaviour_mut().request_response.send_request(&peer, BlockRequest::Fetch(url.clone()));
                                    self.pending_requests.insert(request_id, url.clone());
                                }
                            }
                            kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. })) => {
                                println!("Finished getting providers");
                            }
                            kad::QueryResult::GetProviders(Err(e)) => {
                                eprintln!("Failed to get providers: {:?}", e);
                            }
                            kad::QueryResult::StartProviding(Ok(_)) => {
                                println!("Successfully started providing");
                            }
                            kad::QueryResult::StartProviding(Err(e)) => {
                                eprintln!("Failed to start providing: {:?}", e);
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id: _,
                message,
            })) => {
                let cid = String::from_utf8_lossy(&message.data).to_string();
                println!("Received CID via gossip: {}", cid);

                // Check if we have it
                match self.store.get_node(&cid) {
                    Ok(Some(_)) => {
                        println!("Block {} already exists locally", cid);
                    }
                    Ok(None) => {
                        println!("Block {} missing, requesting from {:?}", cid, propagation_source);
                        let request_id = self
                            .swarm
                            .behaviour_mut()
                            .request_response
                            .send_request(&propagation_source, BlockRequest::Fetch(cid.clone()));
                        self.pending_requests.insert(request_id, cid);
                    }
                    Err(e) => eprintln!("Store error: {:?}", e),
                }
            }

            _ => {}
        }
    }

    async fn process_received_block(&mut self, node: dag::DagNode, source_peer: PeerId) {
        let cid = node.id.clone();
        
        // 1. Store locally
        if let Err(e) = self.store.put_node(&node) {
            eprintln!("Failed to store received node: {:?}", e);
            return;
        }
        
        println!("Stored block {}", cid);
        
        // Emit specific events based on type
        if let dag::DagPayload::Message(_) = node.payload {
             let content = self.try_decrypt(&node);
             let _ = self.event_tx.send(AppEvent::MessageReceived(node.clone(), content));
        } else if let dag::DagPayload::Web(ref web) = node.payload {
             let _ = self.event_tx.send(AppEvent::WebPageFetched { url: web.url.clone(), content: Some(web.content.clone()) });
             let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
        } else {
             let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
        }

        // 2. Check for missing parents (Recursive Backfill)
        for parent_cid in node.prev {
            match self.store.get_node(&parent_cid) {
                Ok(None) => {
                    println!("Parent {} missing, requesting from {:?}", parent_cid, source_peer);
                    let request_id = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_request(&source_peer, BlockRequest::Fetch(parent_cid.clone()));
                    self.pending_requests.insert(request_id, parent_cid);
                }
                Ok(Some(_)) => {
                    // We have the parent, good.
                }
                Err(e) => eprintln!("Store error checking parent: {:?}", e),
            }
        }
    }
}

pub async fn init(
    cmd_rx: mpsc::UnboundedReceiver<AppCmd>,
    event_tx: mpsc::UnboundedSender<AppEvent>,
    keypair: Option<Keypair>,
) {
    let store = match Store::new("store.db") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to create store: {:?}", e);
            return;
        }
    };

    match Backend::new(store, cmd_rx, event_tx.clone(), keypair).await {
        Ok(mut backend) => {
            let peer_id = backend.local_peer_id().to_string();
            let _ = event_tx.send(AppEvent::MyIdentity(peer_id));
            backend.run().await
        },
        Err(e) => eprintln!("Failed to create backend: {:?}", e),
    }
}

async fn detect_geohash(event_tx: mpsc::UnboundedSender<AppEvent>) {
    #[derive(serde::Deserialize)]
    struct IpApiResponse {
        lat: f64,
        lon: f64,
    }

    match reqwest::get("http://ip-api.com/json/").await {
        Ok(resp) => {
            match resp.json::<IpApiResponse>().await {
                Ok(loc) => {
                    match geohash::encode(geohash::Coord { x: loc.lon, y: loc.lat }, 5) {
                        Ok(hash) => {
                            println!("Detected geohash: {}", hash);
                            let _ = event_tx.send(AppEvent::GeohashDetected(hash));
                        }
                        Err(e) => eprintln!("Failed to encode geohash: {:?}", e),
                    }
                }
                Err(e) => eprintln!("Failed to parse IP API response: {:?}", e),
            }
        }
        Err(e) => eprintln!("Failed to fetch IP location: {:?}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::dag::{DagNode, DagPayload, PostPayload};
    use std::time::Duration;

    #[tokio::test]
    async fn test_replication() {
        // Setup Node A
        let (cmd_tx_a, _cmd_rx_a) = mpsc::unbounded_channel();
        let (event_tx_a, mut event_rx_a) = mpsc::unbounded_channel();
        let store_a = Store::new_in_memory().unwrap();
        let mut backend_a = Backend::new(store_a, _cmd_rx_a, event_tx_a, Some(Keypair::generate_ed25519())).await.unwrap();
        
        // Setup Node B
        let (cmd_tx_b, _cmd_rx_b) = mpsc::unbounded_channel();
        let (event_tx_b, mut event_rx_b) = mpsc::unbounded_channel();
        let store_b = Store::new_in_memory().unwrap();
        let mut backend_b = Backend::new(store_b, _cmd_rx_b, event_tx_b, Some(Keypair::generate_ed25519())).await.unwrap();

        // Spawn runners
        tokio::spawn(async move {
            backend_a.run().await;
        });
        
        // Wait for A to start listening
        let addr_a = match tokio::time::timeout(Duration::from_secs(5), event_rx_a.recv()).await {
            Ok(Some(AppEvent::Listening(addr))) => addr,
            Ok(Some(_)) => panic!("Expected Listening event"),
            Ok(None) => panic!("Event channel closed"),
            Err(_) => panic!("Timed out waiting for listener"),
        };
        
        println!("Node A listening on {}", addr_a);
        
        // We need to replace 0.0.0.0 with 127.0.0.1 for local dialing if needed
        let addr_a_str = addr_a.replace("0.0.0.0", "127.0.0.1");
        let addr_a: libp2p::Multiaddr = addr_a_str.parse().unwrap();


        
        // Connect B to A
        backend_b.dial(addr_a).expect("Failed to dial A");

        tokio::spawn(async move {
            backend_b.run().await;
        });

        // Wait for connection and mesh
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Publish block on A
        let keypair = Keypair::generate_ed25519();
        let payload = DagPayload::Post(PostPayload { content: "Replication Test".into(), attachments: vec![], geohash: None });
        let node = DagNode::new("post:v1".into(), payload, vec![], &keypair, 0).unwrap();
        
        println!("Publishing block {}", node.id);
        cmd_tx_a.send(AppCmd::PublishBlock(node.clone())).unwrap();

        // Wait for B to receive
        let received = tokio::time::timeout(Duration::from_secs(10), event_rx_b.recv()).await;
        
        match received {
            Ok(Some(AppEvent::BlockReceived(n))) => {
                println!("Node B received block {}", n.id);
                assert_eq!(n.id, node.id);
            }
            Ok(Some(AppEvent::PeerDiscovered(_))) => {
                // Ignore peer discovery events in this simple test
            }
            Ok(Some(_)) => {
                // Ignore other events
            }
            Ok(None) => panic!("Event channel closed"),
            Err(_) => panic!("Timed out waiting for block"),
        }
    }

    #[tokio::test]
    async fn test_profile_publishing() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        cmd_tx.send(AppCmd::PublishProfile {
            name: "Alice".to_string(),
            bio: "Wonderland".to_string(),
        }).unwrap();

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_vouching() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, _event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        let target_id = "1234567890abcdef".to_string();
        cmd_tx.send(AppCmd::Vouch {
            target_peer_id: target_id.clone(),
        }).unwrap();

        // Allow some time for processing
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test]
    async fn test_web_page_publishing() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        // 1. Publish a page
        let url = "sp://test.super/home".to_string();
        let content = "<h1>Hello World</h1>".to_string();
        
        // We need to be verified to publish. 
        // In test environment, we are not verified by default unless we are founder (first 100).
        // Backend::new generates a new identity.
        // store.count_unique_profiles() will be 0.
        // So if we publish profile, we become founder #1.
        
        cmd_tx.send(AppCmd::PublishProfile {
            name: "Founder".to_string(),
            bio: "First".to_string(),
        }).unwrap();
        
        // Wait for profile to be processed
        tokio::time::sleep(Duration::from_secs(1)).await;

        cmd_tx.send(AppCmd::PublishWebPage {
            url: url.clone(),
            title: "Test Page".to_string(),
            content: content.clone(),
            description: "Test Description".to_string(),
            tags: vec![],
        }).unwrap();

        // Wait for block received event
        let _ = tokio::time::timeout(Duration::from_secs(1), event_rx.recv()).await;

        // 2. Fetch the page
        cmd_tx.send(AppCmd::FetchWebPage {
            url: url.clone(),
        }).unwrap();

        // 3. Verify we get the content back
        // We might receive other events first (BlockReceived, etc.), so we need to loop or filter.
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(2) {
                panic!("Timed out waiting for WebPageFetched");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                if let AppEvent::WebPageFetched { url: u, content: c } = event {
                    assert_eq!(u, url);
                    assert_eq!(c, Some(content.clone()));
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_sns_registration_and_resolution() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        // Become founder
        cmd_tx.send(AppCmd::PublishProfile {
            name: "Founder".to_string(),
            bio: "First".to_string(),
        }).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;

        let name = "alice.super".to_string();
        let target = "sp://alice.super/home".to_string();

        // 1. Register Name
        cmd_tx.send(AppCmd::RegisterName {
            name: name.clone(),
            target: target.clone(),
        }).unwrap();

        // Wait for block received
        let _ = tokio::time::timeout(Duration::from_secs(1), event_rx.recv()).await;

        // 2. Resolve Name
        cmd_tx.send(AppCmd::ResolveName {
            name: name.clone(),
        }).unwrap();

        // 3. Verify
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(2) {
                panic!("Timed out waiting for NameResolved");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                if let AppEvent::NameResolved { name: n, target: t } = event {
                    assert_eq!(n, name);
                    assert_eq!(t, Some(target.clone()));
                    break;
                }
            }
        }

        // 4. Publish content for the target
        cmd_tx.send(AppCmd::PublishWebPage {
            url: target.clone(),
            title: "Alice's Home".to_string(),
            content: "<h1>Alice's Home</h1>".to_string(),
            description: "Member page".to_string(),
            tags: vec!["home".to_string()],
        }).unwrap();
        // Wait for block received
        let _ = tokio::time::timeout(Duration::from_secs(1), event_rx.recv()).await;

        // 5. Fetch Web Page using Name
        cmd_tx.send(AppCmd::FetchWebPage {
            url: name.clone(),
        }).unwrap();

        // 6. Verify we get the content
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(2) {
                panic!("Timed out waiting for WebPageFetched via SNS");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                if let AppEvent::WebPageFetched { url: u, content: c } = event {
                    assert_eq!(u, name);
                    assert!(c.expect("Content missing").contains("Alice's Home"));
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_block_fetching_v2() {
        let (cmd_tx_a, _cmd_rx_a) = mpsc::unbounded_channel();
        let (event_tx_a, mut event_rx_a) = mpsc::unbounded_channel();
        let store_a = Store::new_in_memory().unwrap();
        let mut backend_a = Backend::new(store_a, _cmd_rx_a, event_tx_a, Some(Keypair::generate_ed25519())).await.unwrap();
        
        let (cmd_tx_b, cmd_rx_b) = mpsc::unbounded_channel();
        let (event_tx_b, mut event_rx_b) = mpsc::unbounded_channel();
        let store_b = Store::new_in_memory().unwrap();
        let mut backend_b = Backend::new(store_b, cmd_rx_b, event_tx_b, Some(Keypair::generate_ed25519())).await.unwrap();

        let peer_id_a = backend_a.local_peer_id().to_string();

        tokio::spawn(async move {
            backend_a.run().await;
        });

        // Wait for A to start listening
        let addr_a = match tokio::time::timeout(Duration::from_secs(5), event_rx_a.recv()).await {
            Ok(Some(AppEvent::Listening(addr))) => addr,
            _ => panic!("Failed to get listener address"),
        };
        let addr_a_str = addr_a.replace("0.0.0.0", "127.0.0.1");
        let addr_a: libp2p::Multiaddr = format!("{}/p2p/{}", addr_a_str, peer_id_a).parse().unwrap();

        // Connect B to A
        backend_b.dial(addr_a).expect("Failed to dial A");
        tokio::spawn(async move {
            backend_b.run().await;
        });

        // Wait for connection
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Timed out waiting for connection");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                if let AppEvent::PeerConnected(pid) = event {
                    if pid == peer_id_a {
                        break;
                    }
                }
            }
        }

        // 1. Create a block on A (manually, to avoid gossip)
        let keypair = Keypair::generate_ed25519();
        let payload = DagPayload::Post(PostPayload { content: "Secret Block".into(), attachments: vec![], geohash: None });
        let node = DagNode::new("post:v1".into(), payload, vec![], &keypair, 0).unwrap();
        
        // We need to access store_a to put the node, but backend_a owns it.
        // We can use PublishBlock but it gossips. That's fine, we want to test explicit fetch.
        // To ensure B doesn't get it via gossip, we could disconnect gossipsub? 
        // Or just assume gossip might fail or we want to fetch old blocks.
        // Let's just use PublishBlock on A.
        println!("Created block with CID: {}", node.id);
        cmd_tx_a.send(AppCmd::PublishBlock(node.clone())).unwrap();
        
        // Wait a bit
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 2. B requests the block explicitly
        // We already have A's peer ID.
        
        println!("Requesting block: {}", node.id);
        cmd_tx_b.send(AppCmd::FetchBlock {
            cid: node.id.clone(),
            peer_id: Some(peer_id_a),
        }).unwrap();

        // 3. Verify B receives BlockFetched
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Timed out waiting for BlockFetched");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                if let AppEvent::BlockFetched { cid, node: fetched_node } = event {
                    println!("Received BlockFetched event with CID: {}", cid);
                    assert_eq!(cid, node.id);
                    assert_eq!(fetched_node.unwrap().id, node.id);
                    break;
                }
            }
        }
    }
}

    #[tokio::test]
    async fn test_dht_web_discovery() {
        let (cmd_tx_a, cmd_rx_a) = mpsc::unbounded_channel();
        let (event_tx_a, mut event_rx_a) = mpsc::unbounded_channel();
        let store_a = Store::new_in_memory().unwrap();
        let mut backend_a = Backend::new(store_a, cmd_rx_a, event_tx_a, Some(Keypair::generate_ed25519())).await.unwrap();
        
        let (cmd_tx_b, cmd_rx_b) = mpsc::unbounded_channel();
        let (event_tx_b, mut event_rx_b) = mpsc::unbounded_channel();
        let store_b = Store::new_in_memory().unwrap();
        let mut backend_b = Backend::new(store_b, cmd_rx_b, event_tx_b, Some(Keypair::generate_ed25519())).await.unwrap();

        let peer_id_a = backend_a.local_peer_id().to_string();

        tokio::spawn(async move {
            backend_a.run().await;
        });

        // Wait for A to start listening
        let addr_a = match tokio::time::timeout(Duration::from_secs(5), event_rx_a.recv()).await {
            Ok(Some(AppEvent::Listening(addr))) => addr,
            _ => panic!("Failed to get listener address"),
        };
        let addr_a_str = addr_a.replace("0.0.0.0", "127.0.0.1");
        let addr_a: libp2p::Multiaddr = format!("{}/p2p/{}", addr_a_str, peer_id_a).parse().unwrap();

        // Connect B to A
        backend_b.dial(addr_a).expect("Failed to dial A");
        tokio::spawn(async move {
            backend_b.run().await;
        });

        // Wait for connection
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Timed out waiting for connection");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                if let AppEvent::PeerConnected(pid) = event {
                    if pid == peer_id_a {
                        break;
                    }
                }
            }
        }

        // 1. A publishes a web page
        // Need to be verified first (or founder)
        cmd_tx_a.send(AppCmd::PublishProfile {
            name: "Founder".to_string(),
            bio: "First".to_string(),
        }).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;

        let url = "sp://dht.test/page".to_string();
        let content = "<h1>DHT Test</h1>".to_string();
        cmd_tx_a.send(AppCmd::PublishWebPage {
            url: url.clone(),
            title: "DHT Test".to_string(),
            content: content.clone(),
            description: "DHT Test Page".to_string(),
            tags: vec!["dht".to_string()],
        }).unwrap();
        
        // Wait for A to process and announce
        tokio::time::sleep(Duration::from_secs(2)).await;

        // 2. B requests the web page (should use DHT)
        cmd_tx_b.send(AppCmd::FetchWebPage {
            url: url.clone(),
        }).unwrap();

        // 3. Verify B gets the content
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(10) {
                panic!("Timed out waiting for WebPageFetched via DHT");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                if let AppEvent::WebPageFetched { url: u, content: c } = event {
                    assert_eq!(u, url);
                    assert_eq!(c, Some(content.clone()));
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_smart_contract_kv() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        // Use a random keypair
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        // Helper to drain events until we get what we want or timeout
        // We can't use closure easily due to borrowing event_rx, so we just loop inline.

        // 1. Become Founder
        cmd_tx.send(AppCmd::PublishProfile {
            name: "Founder".to_string(),
            bio: "First".to_string(),
        }).unwrap();
        
        // Wait for Profile Block to ensure we are verified
        let mut profile_set = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                if let AppEvent::BlockReceived(node) = event {
                    if node.r#type == "profile:v1" {
                        profile_set = true;
                        break;
                    }
                }
            }
        }
        assert!(profile_set, "Failed to publish profile");

        // 2. Deploy Contract
        let init_params = r#"{"counter": "0"}"#.to_string();
        cmd_tx.send(AppCmd::DeployContract {
            code: "".to_string(), 
            init_params: init_params.clone(),
        }).unwrap();

        // Wait for contract block
        let mut contract_id = "".to_string();
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                 if let AppEvent::BlockReceived(node) = event {
                     if node.r#type == "contract:v1" {
                         contract_id = node.id;
                         break;
                     }
                 }
                 // Ignore other events
             }
        }
        assert!(!contract_id.is_empty(), "Failed to receive contract block");
        println!("Contract deployed: {}", contract_id);

        // 3. Call Contract (Set)
        let set_params = r#"{"key": "foo", "value": "bar"}"#.to_string();
        cmd_tx.send(AppCmd::CallContract {
            contract_id: contract_id.clone(),
            method: "set".to_string(),
            params: set_params,
        }).unwrap();

        // Wait for call block
        let mut call_found = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                 if let AppEvent::BlockReceived(node) = event {
                     if node.r#type == "contract_call:v1" {
                         call_found = true;
                         break;
                     }
                 }
             }
        }
        assert!(call_found, "Failed to receive call block (set)");

        // 4. Fetch State
        cmd_tx.send(AppCmd::FetchContractState {
            contract_id: contract_id.clone(),
        }).unwrap();

        // Verify state
        let mut state_verified = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                 if let AppEvent::ContractStateFetched { contract_id: cid, state } = event {
                     if cid == contract_id {
                         println!("State: {}", state);
                         if state.contains(r#""foo": "bar""#) {
                            state_verified = true;
                            break;
                         }
                     }
                 }
             }
        }
        assert!(state_verified, "State verification failed (set)");

        // 5. Call Contract (Delete)
        let delete_params = r#"{"key": "foo"}"#.to_string();
        cmd_tx.send(AppCmd::CallContract {
            contract_id: contract_id.clone(),
            method: "delete".to_string(),
            params: delete_params,
        }).unwrap();

        // Wait for call block
        let mut delete_found = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                 if let AppEvent::BlockReceived(node) = event {
                     if node.r#type == "contract_call:v1" {
                         delete_found = true;
                         break;
                     }
                 }
             }
        }
        assert!(delete_found, "Failed to receive call block (delete)");

        // 6. Fetch State again
        cmd_tx.send(AppCmd::FetchContractState {
            contract_id: contract_id.clone(),
        }).unwrap();

        // Verify state
        let mut delete_verified = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                 if let AppEvent::ContractStateFetched { contract_id: cid, state } = event {
                     if cid == contract_id {
                         println!("State: {}", state);
                         if !state.contains(r#""foo": "bar""#) {
                            delete_verified = true;
                            break;
                         }
                     }
                 }
             }
        }
        assert!(delete_verified, "State verification failed (delete)");
    }
