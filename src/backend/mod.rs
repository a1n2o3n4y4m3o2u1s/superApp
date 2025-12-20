pub mod network;
pub mod dag;
use dag::DagPayload;
pub mod store;
pub mod identity;
pub mod wasm;
pub mod vm;
use vm::VM;

use libp2p::{
    futures::StreamExt,
    gossipsub,
    identity::Keypair,
    request_response::{self, OutboundRequestId},
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
use rand::Rng;
use tokio::sync::mpsc;
use std::time::Duration;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug)]
pub enum AppCmd {
    Init,
    PublishBlock(dag::DagNode),
    PublishProfile { name: String, bio: String, photo: Option<String> },
    Vouch { target_peer_id: String },
    PublishPost { content: String, attachments: Vec<String>, geohash: Option<String>, announcement: bool },
    PublishBlob { mime_type: String, data: String },
    FetchPosts,
    FetchLocalPosts { geohash_prefix: String },
    SendMessage { recipient: String, content: String, group_id: Option<String> },
    FetchMessages { peer_id: String },
    CreateGroup { name: String, members: Vec<String> },
    FetchGroups,
    FetchGroupMessages { group_id: String },
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
    AnnouncePresence { geohash: String },
    PublishWebPage { url: String, title: String, content: String, description: String, tags: Vec<String> },
    FetchWebPage { url: String },
    RegisterName { name: String, target: String },
    ResolveName { name: String },
    FetchBlock { cid: String, peer_id: Option<String> },
    FetchStorageStats,
    SetStorageQuota { quota_mb: Option<u64> },  // None = unlimited
    FetchStorageQuota,
    CreateListing { title: String, description: String, price: u64, image_cid: Option<String>, category: Option<String> },
    BuyListing { listing_id: String },
    UpdateListingStatus { listing_id: String, status: dag::ListingStatus },
    SearchListings { query: String },
    FetchListings,

    SearchWeb { query: String },
    SearchFiles { query: String },
    DeployContract { code: String, init_params: String },
    CallContract {
        contract_id: String,
        method: String,
        params: String,
    },
    FetchContracts,
    FetchContractState {
        contract_id: String,
    },
    FetchContractHistory { contract_id: String },
    AcceptContract { contract_id: String },
    RejectContract { contract_id: String },
    CancelContract { contract_id: String },
    PayContract { contract_id: String, amount: u64 },
    FetchPendingContracts, // Contracts awaiting my acceptance
    FetchPublicLedger,
    PublishProposal { title: String, description: String, r#type: dag::ProposalType, pinned: bool },
    VoteProposal { proposal_id: String, vote: dag::VoteType },
    FetchProposals,
    FetchProposalVotes { proposal_id: String },
    FetchProposalTally { proposal_id: String },
    // Election commands
    DeclareCandidacy { ministry: dag::Ministry, platform: String },
    VoteForCandidate { candidacy_id: String },
    FetchCandidates,
    FetchCandidateTally { candidacy_id: String },
    FetchReputation { peer_id: String },
    FetchMyWebPages,
    FetchAllWebPages,
    ReportContent { target_id: String, reason: String, details: String },
    FetchReports,
    UploadFile { name: String, mime_type: String, data: Vec<u8> },
    FetchMyFiles,
    InitiateRecall { target_official: String, ministry: dag::Ministry, reason: String },
    VoteRecall { recall_id: String, vote: bool },
    FetchRecalls,
    FetchRecallTally { recall_id: String },
    EscalateReport { report_id: String },
    CastJuryVote { case_id: String, vote: String }, // "Uphold" or "Dismiss"
    PostComment { parent_id: String, content: String },
    FetchComments { parent_id: String },
    LikePost { target_id: String, remove: bool },
    FetchLikes { target_id: String },
    FetchOversightCases,
    FetchJuryDuty, // Fetch cases where I am a juror
    FetchMinistries,
    PublishStory { media_cid: String, caption: String, geohash: Option<String> },
    FetchStories,
    FollowUser { target: String, follow: bool },
    FetchFollowing { target: String },
    FetchFollowers { target: String },
    FetchGivenUserPosts { peer_id: String },
    FetchFollowingPosts,
    FetchTaxRate,
    // Education System
    CreateCourse { title: String, description: String, content: String, category: String, prerequisites: Vec<String> },
    CreateExam { title: String, course_id: Option<String>, questions: Vec<(String, Vec<String>, usize)>, passing_score: u8, certification_type: String },
    SubmitExam { exam_id: String, answers: Vec<usize> },
    FetchCourses,
    FetchExams,
    FetchCertifications { peer_id: String },
    FetchMyCertifications,
    // Verification Application System
    SubmitApplication { name: String, bio: String, photo_cid: Option<String> },
    VoteApplication { application_id: String, approve: bool },
    FetchPendingApplications,
    FetchApplicationVotes { application_id: String },
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
    GroupsFetched(Vec<dag::DagNode>),
    GroupMessagesFetched(Vec<(dag::DagNode, String)>),
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
    #[allow(dead_code)]
    NameResolved { name: String, target: Option<String> },
    StorageStatsFetched { block_count: usize, total_bytes: usize },
    #[allow(dead_code)]
    StorageQuotaFetched { quota_mb: Option<u64>, used_bytes: usize, percent: u8 },
    #[allow(dead_code)]
    StorageWarning { used_percent: u8, message: String },
    LocalPostsFetched(Vec<dag::DagNode>),
    ListingsFetched(Vec<dag::DagNode>),
    WebSearchResults(Vec<dag::DagNode>),
    FileSearchResults(Vec<dag::DagNode>),
    ContractsFetched(Vec<dag::DagNode>),
    ContractStateFetched {
        contract_id: String,
        state: String,
    },
    #[allow(dead_code)]
    ContractHistoryFetched { contract_id: String, history: Vec<dag::DagNode> },
    PendingContractsFetched(Vec<dag::DagNode>),
    PublicLedgerFetched(Vec<dag::DagNode>),
    ProposalsFetched(Vec<dag::DagNode>),
    ProposalVotesFetched { proposal_id: String, votes: Vec<dag::DagNode> },
    /// Vote tally: (yes, no, abstain, petition, unique_voters, status)
    ProposalTallyFetched { proposal_id: String, yes: usize, no: usize, abstain: usize, petition: usize, unique_voters: usize, status: String },
    // Election events
    CandidatesFetched(Vec<dag::DagNode>),
    CandidateTallyFetched { candidacy_id: String, votes: usize },
    ReputationFetched(dag::ReputationDetails),
    MyWebPagesFetched(Vec<dag::DagNode>),
    AllWebPagesFetched(Vec<dag::DagNode>),
    ReportsFetched(Vec<dag::DagNode>),
    FileUploaded(dag::DagNode),
    MyFilesFetched(Vec<dag::DagNode>),
    RecallsFetched(Vec<dag::DagNode>),
    RecallTallyFetched { recall_id: String, remove: usize, keep: usize, unique_voters: usize },
    OversightCasesFetched(Vec<dag::DagNode>),
    JuryDutyFetched(Vec<dag::DagNode>),
    CommentsFetched { parent_id: String, comments: Vec<dag::DagNode> },
    LikesFetched { target_id: String, count: usize, is_liked_by_me: bool },
    MinistriesFetched(Vec<String>),
    StoriesFetched(Vec<dag::DagNode>),
    FollowingFetched(Vec<String>),
    #[allow(dead_code)]
    FollowersFetched(Vec<String>),
    UserPostsFetched(Vec<dag::DagNode>),
    FollowingPostsFetched(Vec<dag::DagNode>),
    TaxRateFetched(u8),
    // Education System
    CoursesFetched(Vec<dag::DagNode>),
    ExamsFetched(Vec<dag::DagNode>),
    CertificationsFetched(Vec<dag::DagNode>),
    ExamSubmitted { exam_id: String, score: u8, passed: bool },
    // Application Verification System
    PendingApplicationsFetched(Vec<dag::DagNode>),
    #[allow(dead_code)]
    ApplicationVotesFetched { application_id: String, approvals: usize, rejections: usize, required: usize },
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
    current_geohash: Option<String>,
    last_heartbeat: std::time::Instant,
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
            current_geohash: None,
            last_heartbeat: std::time::Instant::now(),
        })
    }

    pub async fn run(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            println!("Attempting to dial bootstrap nodes...");
            for addr_str in network::BOOTSTRAP_NODES.iter() {
                if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
                    match self.swarm.dial(addr.clone()) {
                        Ok(_) => println!("Dialed bootstrap node: {}", addr),
                        Err(e) => eprintln!("Failed to dial bootstrap node {}: {:?}", addr, e),
                    }
                }
            }
            if let Err(e) = self.swarm.behaviour_mut().kad.bootstrap() {
                 eprintln!("Failed to bootstrap Kademlia: {:?}", e);
            }
        }

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
                    
                    // Heartbeat (every 60s)
                    if self.last_heartbeat.elapsed() > std::time::Duration::from_secs(60) {
                         if let Some(gh) = &self.current_geohash {
                             let topic = gossipsub::IdentTopic::new(format!("geohash:{}", gh));
                             let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, "PRESENCE".as_bytes());
                         }
                         self.last_heartbeat = std::time::Instant::now();
                    }
                    
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Err(e) = self.store.prune_expired_stories() {
                             eprintln!("Failed to prune stories: {:?}", e);
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn dial(&mut self, addr: libp2p::Multiaddr) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.dial(addr)?;
        Ok(())
    }

    pub fn local_peer_id(&self) -> PeerId {
        *self.swarm.local_peer_id()
    }

    #[allow(dead_code)]
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

        // 2. Check for approved application
        if let Ok(apps) = self.store.get_pending_applications() {
            for app in apps {
                if app.author == peer_id {
                    // Check if this application has enough approvals
                    if let Ok(votes) = self.store.get_application_votes(&app.id) {
                        let network_size = self.store.count_unique_profiles().unwrap_or(0);
                        let required = Self::required_approvals(network_size);
                        let approvals = votes.iter()
                            .filter(|v| {
                                if let dag::DagPayload::ApplicationVote(ref av) = v.payload {
                                    av.approve
                                } else { false }
                            }).count();
                        if approvals >= required {
                            return true;
                        }
                    }
                }
            }
        }

        // 3. Check proofs (legacy vouch chain - still valid)
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
        let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
        let mut visited = std::collections::HashSet::new();
        self.is_verified(&author_hex, &mut visited)
    }

    /// Calculate required approvals based on network size
    fn required_approvals(network_size: usize) -> usize {
        match network_size {
            0..=100 => 1,      // Bootstrap: 1 vote
            101..=1000 => 3,   // Small: 3 votes
            1001..=10000 => 5, // Medium: 5 votes
            _ => 10,           // Large: 10 votes
        }
    }

    fn has_certification(&self, peer_id: &str, cert_type: &str) -> bool {
        match self.store.get_certifications(peer_id) {
            Ok(nodes) => {
                for node in nodes {
                    if let dag::DagPayload::Certification(cert) = node.payload {
                        if cert.certification_type == cert_type {
                            return true;
                        }
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    fn try_decrypt(&self, node: &dag::DagNode) -> String {
        if let dag::DagPayload::Message(msg) = &node.payload {
            // If I am the recipient
            let my_pubkey = self.keypair.public();
            let my_hex = libp2p::PeerId::from_public_key(&my_pubkey).to_string();
            
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
                                let nonce = Nonce::from_slice(&nonce_bytes);
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
        let target_peers = connected_peers.clone();

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

    async fn process_publish_post(&mut self, content: String, attachments: Vec<String>, geohash: Option<String>, announcement: bool) {
        if !self.is_caller_verified() {
            eprintln!("Cannot publish post: User is not verified.");
            return;
        }
        
        // Permission check for announcements
        if announcement {
            let author_pubkey = self.keypair.public();
            let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
            let officials = self.store.get_active_officials().unwrap_or_default();
            if !officials.values().any(|p| p == &author_hex) {
                eprintln!("Cannot publish announcement: User is not an elected official.");
                return;
            }
        }
        
        let payload = dag::DagPayload::Post(dag::PostPayload { content, attachments, geohash, announcement });
        
        // Get previous head for this user if any
        let author_pubkey = self.keypair.public();
        let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
        
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

    async fn process_send_message(&mut self, recipient: String, content: String, group_id: Option<String>) {
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
                        group_id: group_id.clone(),
                    });

                    // Get previous head for this user if any
                    let author_pubkey = self.keypair.public();
                    let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                    
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
                            
                            // 4. Replicate
                            self.replicate_block(&node);
                        }
                        Err(e) => eprintln!("Failed to create message node: {:?}", e),
                    }
                }
                Err(e) => eprintln!("Failed to encrypt message: {:?}", e),
            }
        } else {
            eprintln!("Recipient has no public key published or invalid.");
        }
    }

    async fn process_publish_profile(&mut self, name: String, bio: String, photo: Option<String>) {
        let author_pubkey = self.keypair.public();
        let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
        
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

        let payload = dag::DagPayload::Profile(dag::ProfilePayload { name, bio, founder_id, encryption_pubkey, photo });
        
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

    async fn process_vouch(&mut self, target_peer_id: String) {
        if !self.is_caller_verified() {
            eprintln!("Cannot vouch: User is not verified.");
            return;
        }

        let author_pubkey = self.keypair.public();
        let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();

        // Check 1: Prevent self-vouch
        if target_peer_id == author_hex {
            eprintln!("Cannot vouch for yourself.");
            return;
        }

        // Check 2: Prevent duplicate vouch
        if let Ok(existing_proofs) = self.store.get_proofs(&target_peer_id) {
            if existing_proofs.iter().any(|p| p.author == author_hex) {
                eprintln!("Cannot vouch: You have already vouched for this user.");
                return;
            }
        }


        let payload = dag::DagPayload::Proof(dag::ProofPayload { target_pubkey: target_peer_id });
        
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
            AppCmd::PublishProfile { name, bio, photo } => {
                self.process_publish_profile(name, bio, photo).await;
            }
            AppCmd::Vouch { target_peer_id } => {
                self.process_vouch(target_peer_id).await;
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
            AppCmd::PublishPost { content, attachments, geohash, announcement } => {
                self.process_publish_post(content, attachments, geohash, announcement).await;
            }
            AppCmd::PublishBlob { mime_type, data } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish blob: User is not verified.");
                    return;
                }
                let data_clone = data.clone();
                let payload = dag::DagPayload::Blob(dag::BlobPayload { mime_type, data });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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
                        // Store in dedicated blob storage
                        if let Ok(bytes) = general_purpose::STANDARD.decode(&data_clone) {
                            if let Err(e) = self.store.put_blob(&node.id, &bytes) {
                                eprintln!("Failed to put blob: {:?}", e);
                            }
                        }
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
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
            AppCmd::SendMessage { recipient, content, group_id } => {
                self.process_send_message(recipient, content, group_id).await;
            }
            AppCmd::PublishStory { media_cid, caption, geohash } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish story: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Story(dag::StoryPayload { media_cid, caption, geohash });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "story:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {

                        println!("Created story node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store story node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish story CID: {:?}", e);
                        }
                        
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create story node: {:?}", e),
                }
            }
            AppCmd::FetchStories => {
                match self.store.get_recent_stories(50) {
                    Ok(stories) => {
                        let _ = self.event_tx.send(AppEvent::StoriesFetched(stories));
                    }
                    Err(e) => eprintln!("Failed to fetch stories: {:?}", e),
                }
            }
            AppCmd::FollowUser { target, follow } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot follow user: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Follow(dag::FollowPayload { target, follow });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "follow:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created follow node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store follow node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish follow CID: {:?}", e);
                        }
                        
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create follow node: {:?}", e),
                }
            }
            AppCmd::FetchFollowing { target } => {
                match self.store.get_following(&target) {
                    Ok(following) => {
                        let _ = self.event_tx.send(AppEvent::FollowingFetched(following));
                    }
                    Err(e) => eprintln!("Failed to fetch following: {:?}", e),
                }
            }
            AppCmd::FetchFollowers { target } => {
                match self.store.get_followers(&target) {
                    Ok(followers) => {
                        let _ = self.event_tx.send(AppEvent::FollowersFetched(followers));
                    }
                    Err(e) => eprintln!("Failed to fetch followers: {:?}", e),
                }
            }
            AppCmd::FetchGivenUserPosts { peer_id } => {
                match self.store.get_posts_by_author(&peer_id, 50) {
                    Ok(posts) => {
                         let _ = self.event_tx.send(AppEvent::UserPostsFetched(posts));
                    }
                     Err(e) => eprintln!("Failed to fetch user posts: {:?}", e),
                }
            }
            AppCmd::FetchFollowingPosts => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_following_posts(&author_hex, 50) {
                    Ok(posts) => {
                         let _ = self.event_tx.send(AppEvent::FollowingPostsFetched(posts));
                    }
                    Err(e) => eprintln!("Failed to fetch following posts: {:?}", e),
                }
            }

            // ========== EDUCATION SYSTEM ==========
            AppCmd::CreateCourse { title, description, content, category, prerequisites } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot create course: User is not verified.");
                    return;
                }
                
                let cat = match category.as_str() {
                    "CivicLiteracy" => dag::CourseCategory::CivicLiteracy,
                    "GovernanceRoles" => dag::CourseCategory::GovernanceRoles,
                    "TechnicalSkills" => dag::CourseCategory::TechnicalSkills,
                    "TradeQualifications" => dag::CourseCategory::TradeQualifications,
                    "ModerationJury" => dag::CourseCategory::ModerationJury,
                    _ => dag::CourseCategory::Custom(category),
                };
                
                let payload = dag::DagPayload::Course(dag::CoursePayload {
                    title,
                    description,
                    content,
                    category: cat,
                    exam_id: None,
                    prerequisites,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };
                
                match dag::DagNode::new("course:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store course node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create course node: {:?}", e),
                }
            }
            
            AppCmd::CreateExam { title, course_id, questions, passing_score, certification_type } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot create exam: User is not verified.");
                    return;
                }
                
                // Convert questions to ExamQuestion structs
                let exam_questions: Vec<dag::ExamQuestion> = questions.into_iter().map(|(q, opts, correct_idx)| {
                    // Hash the correct answer index for verification
                    use sha2::{Sha256, Digest};
                    let mut hasher = Sha256::new();
                    hasher.update(correct_idx.to_string().as_bytes());
                    let hash = hex::encode(hasher.finalize());
                    
                    dag::ExamQuestion {
                        question: q,
                        options: opts,
                        correct_answer_hash: hash,
                    }
                }).collect();
                
                let payload = dag::DagPayload::Exam(dag::ExamPayload {
                    title,
                    course_id,
                    questions: exam_questions,
                    passing_score,
                    certification_type,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(_) => vec![],
                };
                
                match dag::DagNode::new("exam:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store exam node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create exam node: {:?}", e),
                }
            }
            
            AppCmd::SubmitExam { exam_id, answers } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot submit exam: User is not verified.");
                    return;
                }
                
                // Fetch the exam to verify answers
                let (score, passed, cert_type) = match self.store.get_node(&exam_id) {
                    Ok(Some(node)) => {
                        if let dag::DagPayload::Exam(exam) = &node.payload {
                            let mut correct = 0;
                            for (i, answer) in answers.iter().enumerate() {
                                if i < exam.questions.len() {
                                    use sha2::{Sha256, Digest};
                                    let mut hasher = Sha256::new();
                                    hasher.update(answer.to_string().as_bytes());
                                    let hash = hex::encode(hasher.finalize());
                                    if hash == exam.questions[i].correct_answer_hash {
                                        correct += 1;
                                    }
                                }
                            }
                            let score = if !exam.questions.is_empty() {
                                ((correct as f32 / exam.questions.len() as f32) * 100.0) as u8
                            } else {
                                0
                            };
                            let passed = score >= exam.passing_score;
                            (score, passed, exam.certification_type.clone())
                        } else {
                            eprintln!("Node is not an exam");
                            return;
                        }
                    }
                    _ => {
                        eprintln!("Exam not found: {}", exam_id);
                        return;
                    }
                };
                
                let payload = dag::DagPayload::ExamSubmission(dag::ExamSubmissionPayload {
                    exam_id: exam_id.clone(),
                    answers,
                    score,
                    passed,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(_) => vec![],
                };
                
                match dag::DagNode::new("exam_submission:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store exam submission: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic.clone(), node.id.as_bytes());
                        let _ = self.event_tx.send(AppEvent::ExamSubmitted { exam_id: exam_id.clone(), score, passed });
                        self.replicate_block(&node);
                        
                        // Issue Certification if Passed
                        if passed {
                             println!("Exam passed ({}%), issuing certification...", score);
                             
                             let cert_payload = dag::DagPayload::Certification(dag::CertificationPayload {
                                 recipient: author_hex.clone(),
                                 certification_type: cert_type.clone(),
                                 exam_id: Some(exam_id.clone()),
                                 issuer_signatures: vec![],
                                 issued_at: chrono::Utc::now(), 
                                 expires_at: None,
                             });
                             
                             // Get updated head
                             let cert_prev = match self.store.get_head(&author_hex) {
                                 Ok(Some(cid)) => vec![cid],
                                 _ => vec![],
                             };
                             
                             match dag::DagNode::new("certification:v1".to_string(), cert_payload, cert_prev, &self.keypair, 0) {
                                 Ok(cert_node) => {
                                     println!("Created certification node: {}", cert_node.id);
                                     if let Err(e) = self.store.put_node(&cert_node) {
                                         eprintln!("Failed to store cert node: {:?}", e);
                                     } else {
                                         if let Err(e) = self.store.update_head(&author_hex, &cert_node.id) {
                                             eprintln!("Failed to update head: {:?}", e);
                                         }
                                         let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, cert_node.id.as_bytes());
                                         let _ = self.event_tx.send(AppEvent::BlockReceived(cert_node.clone()));
                                         self.replicate_block(&cert_node);
                                     }
                                 }
                                 Err(e) => eprintln!("Failed to create cert node: {:?}", e),
                             }
                        }
                    }
                    Err(e) => eprintln!("Failed to create exam submission node: {:?}", e),
                }
            }
            
            AppCmd::FetchCourses => {
                match self.store.get_courses(50) {
                    Ok(courses) => {
                        let _ = self.event_tx.send(AppEvent::CoursesFetched(courses));
                    }
                    Err(e) => eprintln!("Failed to fetch courses: {:?}", e),
                }
            }
            
            AppCmd::FetchExams => {
                match self.store.get_exams(50) {
                    Ok(exams) => {
                        let _ = self.event_tx.send(AppEvent::ExamsFetched(exams));
                    }
                    Err(e) => eprintln!("Failed to fetch exams: {:?}", e),
                }
            }
            
            AppCmd::FetchCertifications { peer_id } => {
                match self.store.get_certifications(&peer_id) {
                    Ok(certs) => {
                        let _ = self.event_tx.send(AppEvent::CertificationsFetched(certs));
                    }
                    Err(e) => eprintln!("Failed to fetch certifications: {:?}", e),
                }
            }
            
            AppCmd::FetchMyCertifications => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_certifications(&author_hex) {
                    Ok(certs) => {
                        let _ = self.event_tx.send(AppEvent::CertificationsFetched(certs));
                    }
                    Err(e) => eprintln!("Failed to fetch my certifications: {:?}", e),
                }
            }

            // === Application-Based Verification System ===
            AppCmd::SubmitApplication { name, bio, photo_cid } => {
                // Anyone can submit an application
                let payload = dag::DagPayload::Application(dag::ApplicationPayload {
                    name,
                    bio,
                    photo_cid,
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);

                match dag::DagNode::new("application:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Created application node: {}", node.id);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to create application: {:?}", e),
                }
            }

            AppCmd::VoteApplication { application_id, approve } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot vote: User is not verified.");
                    return;
                }

                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();

                // Check cooldown (12 hours between votes)
                if let Ok(Some(last_vote)) = self.store.get_latest_application_vote_time(&author_hex) {
                    let hours_since = (chrono::Utc::now() - last_vote).num_hours();
                    if hours_since < 12 {
                        eprintln!("Cannot vote: Cooldown active. {} hours remaining.", 12 - hours_since);
                        return;
                    }
                }

                // Check for duplicate vote on same application
                if let Ok(existing_votes) = self.store.get_application_votes(&application_id) {
                    if existing_votes.iter().any(|v| v.author == author_hex) {
                        eprintln!("Cannot vote: Already voted on this application.");
                        return;
                    }
                }

                let payload = dag::DagPayload::ApplicationVote(dag::ApplicationVotePayload {
                    application_id: application_id.clone(),
                    approve,
                });
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);

                match dag::DagNode::new("application_vote:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Voted on application {}: approve={}", application_id, approve);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to vote on application: {:?}", e),
                }
            }

            AppCmd::FetchPendingApplications => {
                match self.store.get_pending_applications() {
                    Ok(apps) => {
                        let _ = self.event_tx.send(AppEvent::PendingApplicationsFetched(apps));
                    }
                    Err(e) => eprintln!("Failed to fetch pending applications: {:?}", e),
                }
            }

            AppCmd::FetchApplicationVotes { application_id } => {
                // Count votes and determine required threshold
                let network_size = self.store.count_unique_profiles().unwrap_or(0);
                let required = Self::required_approvals(network_size);
                
                match self.store.get_application_votes(&application_id) {
                    Ok(votes) => {
                        let approvals = votes.iter()
                            .filter(|v| {
                                if let dag::DagPayload::ApplicationVote(ref av) = v.payload {
                                    av.approve
                                } else { false }
                            }).count();
                        let rejections = votes.len() - approvals;
                        let _ = self.event_tx.send(AppEvent::ApplicationVotesFetched {
                            application_id, approvals, rejections, required
                        });
                    }
                    Err(e) => eprintln!("Failed to fetch application votes: {:?}", e),
                }
            }

            AppCmd::FetchMyProfile => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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
                        if profile.is_none() {
                             // Attempt to discover peer on network
                             println!("Profile not found locally, querying network for {}", peer_id);
                             if let Ok(pid) = peer_id.parse::<PeerId>() {
                                  self.swarm.behaviour_mut().kad.get_closest_peers(pid.to_bytes());
                             }
                        }
                        let _ = self.event_tx.send(AppEvent::UserProfileFetched(profile));
                    }
                    Err(e) => {
                        eprintln!("Failed to fetch user profile: {:?}", e);
                        let _ = self.event_tx.send(AppEvent::UserProfileFetched(None));
                    }
                }
            }

            AppCmd::AnnouncePresence { geohash } => {
                println!("Announcing presence in {}", geohash);
                self.current_geohash = Some(geohash.clone());
                let topic = gossipsub::IdentTopic::new(format!("geohash:{}", geohash));
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                     eprintln!("Failed to subscribe to geohash: {:?}", e);
                }
                // Publish initial presence
                let presence_msg = "PRESENCE".as_bytes();
                if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, presence_msg) {
                     eprintln!("Failed to publish presence: {:?}", e);
                }
            }



            AppCmd::FetchTaxRate => {
                match self.store.get_current_tax_rate() {
                    Ok(rate) => {
                        let _ = self.event_tx.send(AppEvent::TaxRateFetched(rate));
                    }
                    Err(e) => eprintln!("Failed to fetch tax rate: {:?}", e),
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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

                // 1. Calculate Tax
                let tax_rate = self.store.get_current_tax_rate().unwrap_or(0);
                let tax_amount = if tax_rate > 0 {
                    (amount as u128 * tax_rate as u128 / 100) as u64
                } else {
                    0
                };
                let recipient_amount = amount - tax_amount;

                // 2. Create Transfer Node (to recipient)
                let transfer_payload = dag::DagPayload::Token(dag::TokenPayload {
                    action: dag::TokenAction::Burn,
                    amount: recipient_amount,
                    target: Some(recipient.clone()),
                    memo: Some("Transfer".to_string()),
                    ref_cid: None,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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
                    transfer_payload,
                    prev.clone(),
                    &self.keypair,
                    0
                ) {
                    Ok(transfer_node) => {
                        println!("Created transfer node: {}", transfer_node.id);
                        // Publish Transfer Node
                        if let Err(e) = self.store.put_node(&transfer_node) {
                            eprintln!("Failed to store node {}: {:?}", transfer_node.id, e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &transfer_node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, transfer_node.id.as_bytes());
                        let _ = self.event_tx.send(AppEvent::BlockReceived(transfer_node.clone()));
                        self.replicate_block(&transfer_node);

                        // 3. Create Tax Node (if applicable)
                        if tax_amount > 0 {
                            let tax_payload = dag::DagPayload::Token(dag::TokenPayload {
                                action: dag::TokenAction::Burn,
                                amount: tax_amount,
                                target: None, // Burn to network (System Tax)
                                memo: Some(format!("Tax ({}%)", tax_rate)),
                                ref_cid: Some(transfer_node.id.clone()), // Link to the transfer
                            });
                            
                            // Chain off the transfer node
                            let tax_prev = vec![transfer_node.id.clone()];

                            match dag::DagNode::new(
                                "token:v1".to_string(),
                                tax_payload,
                                tax_prev,
                                &self.keypair,
                                0
                            ) {
                                Ok(tax_node) => {
                                    println!("Created tax node: {}", tax_node.id);
                                    // Publish Tax Node
                                    if let Err(e) = self.store.put_node(&tax_node) {
                                        eprintln!("Failed to store node {}: {:?}", tax_node.id, e);
                                    } else {
                                        if let Err(e) = self.store.update_head(&author_hex, &tax_node.id) {
                                            eprintln!("Failed to update head: {:?}", e);
                                        }
                                        let topic = gossipsub::IdentTopic::new("blocks");
                                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, tax_node.id.as_bytes());
                                        let _ = self.event_tx.send(AppEvent::BlockReceived(tax_node.clone()));
                                        self.replicate_block(&tax_node);
                                    }
                                }
                                Err(e) => eprintln!("Failed to create tax node: {:?}", e),
                            }
                        }
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
                                 let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                                 
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_pending_transfers(&author_hex) {
                    Ok(pending) => {
                        let _ = self.event_tx.send(AppEvent::PendingTransfersFetched(pending));
                    }
                    Err(e) => eprintln!("Failed to fetch pending transfers: {:?}", e),
                }
            }

            AppCmd::FetchBalance => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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

                let author_hex = self.local_peer_id().to_string();
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
                 println!("Starting DHT search for: {}", query);
                 let key = kad::RecordKey::new(&format!("search:term:{}", query).into_bytes());
                 self.swarm.behaviour_mut().kad.get_providers(key);
            }
            AppCmd::SearchFiles { query } => {
                match self.store.search_files(&query) {
                    Ok(results) => {
                         let _ = self.event_tx.send(AppEvent::FileSearchResults(results));
                    }
                    Err(e) => eprintln!("Failed to search files: {:?}", e),
                }
            }
            AppCmd::DeployContract { code, init_params } => {
                 if !self.is_caller_verified() {
                    eprintln!("Cannot deploy contract: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Contract(dag::ContractPayload { 
                    code, 
                    init_params,
                    status: dag::ContractStatus::Pending,
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
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

            AppCmd::FetchContractHistory { contract_id } => {
                match self.store.get_nodes_by_ref(&contract_id) {
                    Ok(history) => {
                        let _ = self.event_tx.send(AppEvent::ContractHistoryFetched { contract_id, history });
                    }
                     Err(e) => eprintln!("Failed to fetch contract history: {:?}", e),
                }
            }

            AppCmd::AcceptContract { contract_id } => {
                // Create a contract_call node with method "accept" to signal acceptance
                let payload = dag::DagPayload::ContractCall(dag::ContractCallPayload {
                    contract_id: contract_id.clone(),
                    method: "accept".to_string(),
                    params: "{}".to_string(),
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);
                
                match dag::DagNode::new("contract_call:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Contract accepted: {}", contract_id);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to accept contract: {:?}", e),
                }
            }

            AppCmd::RejectContract { contract_id } => {
                let payload = dag::DagPayload::ContractCall(dag::ContractCallPayload {
                    contract_id: contract_id.clone(),
                    method: "reject".to_string(),
                    params: "{}".to_string(),
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);
                
                match dag::DagNode::new("contract_call:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Contract rejected: {}", contract_id);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to reject contract: {:?}", e),
                }
            }

            AppCmd::CancelContract { contract_id } => {
                let payload = dag::DagPayload::ContractCall(dag::ContractCallPayload {
                    contract_id: contract_id.clone(),
                    method: "cancel".to_string(),
                    params: "{}".to_string(),
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);
                
                match dag::DagNode::new("contract_call:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Contract cancelled: {}", contract_id);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to cancel contract: {:?}", e),
                }
            }

            AppCmd::PayContract { contract_id, amount } => {
                // Parse the contract to find the recipient (provider)
                let recipient = match self.store.get_node(&contract_id) {
                    Ok(Some(node)) => {
                        if let dag::DagPayload::Contract(c) = &node.payload {
                            let params: serde_json::Value = serde_json::from_str(&c.init_params).unwrap_or(serde_json::json!({}));
                            params["parties"]["provider"].as_str().unwrap_or("").to_string()
                        } else {
                            "".to_string()
                        }
                    }
                    _ => "".to_string(),
                };

                if recipient.is_empty() {
                    eprintln!("Could not determine recipient from contract");
                    return;
                }

                // Create token transfer with ref_cid pointing to the contract
                let payload = dag::DagPayload::Token(dag::TokenPayload {
                    action: dag::TokenAction::Burn,
                    amount,
                    target: Some(recipient.clone()),
                    memo: Some(format!("Contract payment: {}", contract_id)),
                    ref_cid: Some(contract_id.clone()),
                });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = self.store.get_head(&author_hex).ok().flatten().map_or(vec![], |c| vec![c]);

                match dag::DagNode::new("token:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Contract payment: {} tokens to {} for contract {}", amount, recipient, contract_id);
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node));
                    }
                    Err(e) => eprintln!("Failed to pay contract: {:?}", e),
                }
            }

            AppCmd::FetchPendingContracts => {
                // Fetch contracts where I am the counterparty and status is Pending
                let my_id = libp2p::PeerId::from_public_key(&self.keypair.public()).to_string();
                match self.store.get_contracts() {
                    Ok(contracts) => {
                        let pending: Vec<dag::DagNode> = contracts.into_iter().filter(|n| {
                            if let dag::DagPayload::Contract(c) = &n.payload {
                                if c.status != dag::ContractStatus::Pending { return false; }
                                // Check if I'm the consumer/borrower (counterparty)
                                let params: serde_json::Value = serde_json::from_str(&c.init_params).unwrap_or(serde_json::json!({}));
                                let consumer = params["parties"]["consumer"].as_str().unwrap_or("");
                                consumer == my_id && n.author != my_id // I'm consumer but not creator
                            } else {
                                false
                            }
                        }).collect();
                        let _ = self.event_tx.send(AppEvent::PendingContractsFetched(pending));
                    }
                    Err(e) => eprintln!("Failed to fetch pending contracts: {:?}", e),
                }
            }

            AppCmd::InitiateRecall { target_official, ministry, reason } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot initiate recall: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Recall(dag::RecallPayload { target_official, ministry, reason });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };

                match dag::DagNode::new("recall:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                        println!("Created recall node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) { eprintln!("Failed to store recall: {:?}", e); return; }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) { eprintln!("Failed to update head: {:?}", e); }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create recall node: {:?}", e),
                }
            }

            AppCmd::VoteRecall { recall_id, vote } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot vote on recall: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::RecallVote(dag::RecallVotePayload { recall_id: recall_id.clone(), vote });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };
                
                match dag::DagNode::new("recall_vote:v1".to_string(), payload, prev, &self.keypair, 0) {
                    Ok(node) => {
                         println!("Created recall vote node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) { eprintln!("Failed to store recall vote: {:?}", e); return; }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) { eprintln!("Failed to update head: {:?}", e); }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create recall vote: {:?}", e),
                }
            }

            AppCmd::FetchRecalls => {
                match self.store.get_recalls() {
                    Ok(recalls) => {
                        let _ = self.event_tx.send(AppEvent::RecallsFetched(recalls));
                    }
                    Err(e) => eprintln!("Failed to fetch recalls: {:?}", e),
                }
            }

             AppCmd::FetchRecallTally { recall_id } => {
                match self.store.get_recall_tally(&recall_id) {
                    Ok((remove, keep, unique_voters)) => {
                        let _ = self.event_tx.send(AppEvent::RecallTallyFetched { recall_id, remove, keep, unique_voters });
                    }
                    Err(e) => eprintln!("Failed to fetch recall tally: {:?}", e),
                }
            }

            AppCmd::EscalateReport { report_id } => {
                if !self.is_caller_verified() { return; }
                
                // Simple Jury Selection: Get all profiles, filter verified, pick 3 random
                let mut candidates = Vec::new();
                if let Ok(nodes) = self.store.get_all_nodes() {
                    let mut seen_profiles = std::collections::HashSet::new();
                    for node in nodes {
                        if let dag::DagPayload::Profile(p) = node.payload {
                            if !seen_profiles.contains(&node.author) {
                                // check verification status (mock logic: if they have a profile, check if verified)
                                // Ideally we check Verification payload, but for now let's assume all distinct profiles are candidates
                                // REAL implementation needs to check VerificationStatus
                                seen_profiles.insert(node.author.clone()); 
                                candidates.push(node.author);
                            }
                        }
                    }
                }
                
                // Randomly select 3
                use rand::seq::SliceRandom;
                let mut rng = rand::thread_rng();
                candidates.shuffle(&mut rng);
                let jury_members: Vec<String> = candidates.into_iter().take(3).collect();
                
                let mut id_bytes = [0u8; 16];
                rng.fill(&mut id_bytes);
                let case_id = hex::encode(id_bytes);
                let payload = dag::DagPayload::OversightCase(dag::OversightCasePayload {
                    case_id: case_id.clone(),
                    report_id,
                    jury_members,
                    status: "Open".to_string(),
                });

                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = match self.store.get_head(&author_hex) { Ok(Some(cid)) => vec![cid], _ => vec![] };
                
                if let Ok(node) = dag::DagNode::new("oversight_case:v1".to_string(), payload, prev, &self.keypair, 0) {
                     let _ = self.store.put_node(&node);
                     let _ = self.store.update_head(&author_hex, &node.id);
                     let topic = gossipsub::IdentTopic::new("blocks");
                     let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                     let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                     self.replicate_block(&node);
                }
            }

            AppCmd::CastJuryVote { case_id, vote } => {
                if !self.is_caller_verified() { return; }
                // Verify user is on jury logic omitted for brevity/speed, rely on UI + honest clients for MVP
                
                let payload = dag::DagPayload::JuryVote(dag::JuryVotePayload { case_id, vote });
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                let prev = match self.store.get_head(&author_hex) { Ok(Some(cid)) => vec![cid], _ => vec![] };
                
                if let Ok(node) = dag::DagNode::new("jury_vote:v1".to_string(), payload, prev, &self.keypair, 0) {
                     let _ = self.store.put_node(&node);
                     let _ = self.store.update_head(&author_hex, &node.id);
                     let topic = gossipsub::IdentTopic::new("blocks");
                     let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                     self.replicate_block(&node);
                }
            }

            AppCmd::FetchOversightCases => {
                if let Ok(cases) = self.store.get_oversight_cases() {
                    let _ = self.event_tx.send(AppEvent::OversightCasesFetched(cases));
                }
            }

            AppCmd::FetchJuryDuty => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                if let Ok(cases) = self.store.get_user_jury_duty(&author_hex) {
                    let _ = self.event_tx.send(AppEvent::JuryDutyFetched(cases));
                }
            }

            AppCmd::FetchPublicLedger => {
                match self.store.get_public_ledger_events(50) {
                    Ok(events) => {
                        let _ = self.event_tx.send(AppEvent::PublicLedgerFetched(events));
                    }
                    Err(e) => eprintln!("Failed to fetch public ledger: {:?}", e),
                }
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
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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
                    Ok(stats) => {
                        let _ = self.event_tx.send(AppEvent::StorageStatsFetched { 
                            block_count: stats.total_nodes, 
                            total_bytes: stats.total_bytes 
                        });
                    }
                    Err(e) => eprintln!("Failed to get storage stats: {:?}", e),
                }
            }

            AppCmd::SetStorageQuota { quota_mb } => {
                let quota_bytes = quota_mb.map(|mb| mb * 1024 * 1024);
                match self.store.set_storage_quota(quota_bytes) {
                    Ok(()) => {
                        // Fetch and emit updated quota status
                            if let Ok((used, _quota, percent, _)) = self.store.check_storage_quota() {
                            let _ = self.event_tx.send(AppEvent::StorageQuotaFetched {
                                quota_mb,
                                used_bytes: used,
                                percent,
                            });
                        }
                    }
                    Err(e) => eprintln!("Failed to set storage quota: {:?}", e),
                }
            }

            AppCmd::FetchStorageQuota => {
                match self.store.check_storage_quota() {
                    Ok((used, quota, percent, over_quota)) => {
                        let quota_mb = quota.map(|b| b / 1024 / 1024);
                        let _ = self.event_tx.send(AppEvent::StorageQuotaFetched {
                            quota_mb,
                            used_bytes: used,
                            percent,
                        });
                        // Emit warning if over 80%
                        if percent >= 80 {
                            let msg = if over_quota {
                                format!("Storage quota exceeded! {}% used", percent)
                            } else {
                                format!("Storage usage high: {}% of quota used", percent)
                            };
                            let _ = self.event_tx.send(AppEvent::StorageWarning {
                                used_percent: percent,
                                message: msg,
                            });
                        }
                    }
                    Err(e) => eprintln!("Failed to check storage quota: {:?}", e),
                }
            }

            AppCmd::CreateListing { title, description, price, image_cid, category } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot create listing: User is not verified.");
                    return;
                }
                
                if let Some(cat) = &category {
                     let author_pubkey = self.keypair.public();
                     let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                     if !self.has_certification(&author_hex, cat) {
                         eprintln!("Cannot create listing in category '{}': Certification missing", cat);
                         return;
                     }
                }

                let payload = dag::DagPayload::Listing(dag::ListingPayload {
                    title,
                    description,
                    price,
                    image_cid,
                    category,
                    status: dag::ListingStatus::Active,
                    ref_cid: None,
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
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

            AppCmd::BuyListing { listing_id } => {
                if !self.is_caller_verified() {
                     eprintln!("Cannot buy item: User is not verified.");
                     return;
                }
                // 1. Fetch listing to get price and seller
                 match self.store.get_node(&listing_id) {
                     Ok(Some(node)) => {
                         if let dag::DagPayload::Listing(listing) = node.payload {
                             // 2. Initiate Transfer
                             let memo = format!("Purchase: {}", listing_id);
                             let recipient = node.author.clone(); // Seller
                             
                             // Reuse SendToken logic essentially
                             let payload = dag::DagPayload::Token(dag::TokenPayload {
                                action: dag::TokenAction::Burn, // Transfer is a burn targeted at someone
                                amount: listing.price,
                                target: Some(recipient),
                                memo: Some(memo),
                                ref_cid: None,
                            });
                            
                            let author_pubkey = self.keypair.public();
                            let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                             let prev = match self.store.get_head(&author_hex) {
                                Ok(Some(cid)) => vec![cid],
                                _ => vec![],
                            };
                            
                            match dag::DagNode::new("token:v1".to_string(), payload, prev, &self.keypair, 0) {
                                Ok(tnode) => {
                                    println!("Created purchase transfer node: {}", tnode.id);
                                    let _ = self.store.put_node(&tnode);
                                    let _ = self.store.update_head(&author_hex, &tnode.id);
                                     let topic = gossipsub::IdentTopic::new("blocks");
                                    let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, tnode.id.as_bytes());
                                     let _ = self.event_tx.send(AppEvent::BlockReceived(tnode.clone()));
                                     self.replicate_block(&tnode);
                                     // Refresh balance
                                     let _ = self.event_tx.send(AppEvent::BalanceFetched(self.store.get_balance(&author_hex).unwrap_or(0)));
                                }
                                Err(e) => eprintln!("Failed to create purchase node: {:?}", e),
                            }
                         } else {
                             eprintln!("Node {} is not a listing", listing_id);
                         }
                     }
                     _ => eprintln!("Listing {} not found", listing_id),
                 }
            }

            AppCmd::UpdateListingStatus { listing_id, status } => {
                 if !self.is_caller_verified() {
                     eprintln!("Cannot update listing: User is not verified.");
                     return;
                }
                // 1. Fetch original to verify ownership and get details
                 match self.store.get_node(&listing_id) {
                     Ok(Some(node)) => {
                         let author_pubkey = self.keypair.public();
                         let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                         
                         if node.author != author_hex {
                             eprintln!("Cannot update listing: Not the author.");
                             return;
                         }

                         if let dag::DagPayload::Listing(listing) = node.payload {
                             // 2. Create new node with updated status
                             let new_payload = dag::DagPayload::Listing(dag::ListingPayload {
                                 title: listing.title,
                                 description: listing.description,
                                 price: listing.price,
                                 image_cid: listing.image_cid,
                                 category: listing.category.clone(),
                                 status: status,
                                 ref_cid: Some(listing.ref_cid.clone().unwrap_or(listing_id.clone())),
                             });

                             let prev = match self.store.get_head(&author_hex) {
                                Ok(Some(cid)) => vec![cid],
                                _ => vec![],
                            };
                            
                             match dag::DagNode::new("listing:v1".to_string(), new_payload, prev, &self.keypair, 0) {
                                Ok(new_node) => {
                                     println!("Updated listing status: {}", new_node.id);
                                     let _ = self.store.put_node(&new_node);
                                     let _ = self.store.update_head(&author_hex, &new_node.id);
                                     let topic = gossipsub::IdentTopic::new("blocks");
                                     let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, new_node.id.as_bytes());
                                     let _ = self.event_tx.send(AppEvent::BlockReceived(new_node.clone()));
                                     self.replicate_block(&new_node);
                                     // Refresh listings
                                     if let Ok(listings) = self.store.get_active_listings(50) {
                                         let _ = self.event_tx.send(AppEvent::ListingsFetched(listings));
                                     }
                                }
                                Err(e) => eprintln!("Failed to update listing: {:?}", e),
                             }
                         }
                     }
                     _ => eprintln!("Listing not found"),
                 }
            }

            AppCmd::SearchListings { query } => {
                match self.store.search_listings(&query) {
                    Ok(listings) => {
                         let _ = self.event_tx.send(AppEvent::ListingsFetched(listings));
                    }
                    Err(e) => eprintln!("Failed to search listings: {:?}", e),
                }
            }
            AppCmd::PublishProposal { title, description, r#type, pinned } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot publish proposal: User is not verified.");
                    return;
                }

                // Permission check for pinning
                if pinned {
                    let author_pubkey = self.keypair.public();
                    let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                    let officials = self.store.get_active_officials().unwrap_or_default();
                    if !officials.values().any(|p| p == &author_hex) {
                        eprintln!("Cannot pin proposal: User is not an elected official.");
                        return;
                    }
                }

                // Check certifications for specific proposal types
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();

                match r#type {
                    dag::ProposalType::Constitutional | dag::ProposalType::SetTax(_) | dag::ProposalType::DefineMinistries(_) => {
                         if !self.has_certification(&author_hex, "CivicLiteracy") {
                             eprintln!("Cannot publish sensitive proposal: Missing CivicLiteracy certification.");
                             return;
                         }
                    },
                    _ => {}
                }

                let payload = dag::DagPayload::Proposal(dag::ProposalPayload { title, description, r#type, pinned });
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "proposal:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created proposal node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store proposal node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish proposal CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create proposal node: {:?}", e),
                }
            }
            AppCmd::VoteProposal { proposal_id, vote } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot vote: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Vote(dag::VotePayload { proposal_id, vote });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "vote:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created vote node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store vote node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish vote CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create vote node: {:?}", e),
                }
            }
            AppCmd::FetchProposals => {
                match self.store.get_proposals() {
                    Ok(proposals) => {
                        let _ = self.event_tx.send(AppEvent::ProposalsFetched(proposals));
                    }
                    Err(e) => eprintln!("Failed to fetch proposals: {:?}", e),
                }
            }
            AppCmd::FetchProposalVotes { proposal_id } => {
                match self.store.get_votes_for_proposal(&proposal_id) {
                    Ok(votes) => {
                        let _ = self.event_tx.send(AppEvent::ProposalVotesFetched { proposal_id, votes });
                    }
                    Err(e) => eprintln!("Failed to fetch votes for proposal: {:?}", e),
                }
            }
            AppCmd::FetchProposalTally { proposal_id } => {
                let status = self.store.get_proposal_status(&proposal_id).unwrap_or("Unknown".to_string());
                match self.store.get_proposal_vote_tally(&proposal_id) {
                    Ok((yes, no, abstain, petition, unique_voters)) => {
                        let _ = self.event_tx.send(AppEvent::ProposalTallyFetched {
                            proposal_id,
                            yes,
                            no,
                            abstain,
                            petition,
                            unique_voters,
                            status,
                        });
                    }
                    Err(e) => eprintln!("Failed to fetch vote tally: {:?}", e),
                }
            }
            // Election command handlers
            AppCmd::DeclareCandidacy { ministry, platform } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot declare candidacy: User is not verified.");
                    return;
                }
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();

                if !self.has_certification(&author_hex, "GovernanceRoles") {
                    eprintln!("Cannot declare candidacy: Missing 'GovernanceRoles' certification.");
                    return;
                }

                let payload = dag::DagPayload::Candidacy(dag::CandidacyPayload { ministry, platform });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "candidacy:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created candidacy node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store candidacy node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish candidacy CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create candidacy node: {:?}", e),
                }
            }
            AppCmd::VoteForCandidate { candidacy_id } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot vote: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::CandidacyVote(dag::CandidacyVotePayload { candidacy_id });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "candidacy_vote:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created candidacy vote node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store candidacy vote node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish candidacy vote CID: {:?}", e);
                        }
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create candidacy vote node: {:?}", e),
                }
            }
            AppCmd::FetchCandidates => {
                match self.store.get_all_candidates() {
                    Ok(candidates) => {
                        let _ = self.event_tx.send(AppEvent::CandidatesFetched(candidates));
                    }
                    Err(e) => eprintln!("Failed to fetch candidates: {:?}", e),
                }
            }
            AppCmd::FetchCandidateTally { candidacy_id } => {
                match self.store.get_candidate_tally(&candidacy_id) {
                    Ok(votes) => {
                        let _ = self.event_tx.send(AppEvent::CandidateTallyFetched { candidacy_id, votes });
                    }
                    Err(e) => eprintln!("Failed to fetch candidate tally: {:?}", e),
                }
            }
            AppCmd::FetchReputation { peer_id } => {
                match self.store.get_reputation(&peer_id) {
                    Ok(details) => {
                        let _ = self.event_tx.send(AppEvent::ReputationFetched(details));
                    }
                    Err(e) => eprintln!("Failed to fetch reputation: {:?}", e),
                }
            }
            AppCmd::FetchMyWebPages => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_my_web_pages(&author_hex) {
                    Ok(nodes) => {
                        let _ = self.event_tx.send(AppEvent::MyWebPagesFetched(nodes));
                    }
                    Err(e) => eprintln!("Failed to fetch my web pages: {:?}", e),
                }
            }
            
            AppCmd::FetchAllWebPages => {
                match self.store.get_all_web_pages() {
                    Ok(nodes) => {
                        let _ = self.event_tx.send(AppEvent::AllWebPagesFetched(nodes));
                    }
                    Err(e) => eprintln!("Failed to fetch all web pages: {:?}", e),
                }
            }
            
            AppCmd::CreateGroup { name, members } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot create group: User is not verified.");
                    return;
                }
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                // Ensure I am in the members list
                let mut all_members = members.clone();
                if !all_members.contains(&author_hex) {
                    all_members.push(author_hex.clone());
                }

                let payload = dag::DagPayload::Group(dag::GroupPayload { name, members: all_members, owner: author_hex.clone() });
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };

                match dag::DagNode::new(
                    "group:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                         println!("Created group node: {}", node.id);
                         if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store group node: {:?}", e);
                            return;
                        }
                         if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        
                        // Notify UI
                        let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                        
                        // Force refresh of my groups
                        let _ = self.event_tx.send(AppEvent::GroupsFetched(vec![node.clone()]));
                    }
                    Err(e) => eprintln!("Failed to create group node: {:?}", e),
                }
            }

            AppCmd::FetchGroups => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_my_groups(&author_hex) {
                    Ok(groups) => {
                        let _ = self.event_tx.send(AppEvent::GroupsFetched(groups));
                    }
                    Err(e) => eprintln!("Failed to fetch groups: {:?}", e),
                }
            }
            
            AppCmd::FetchGroupMessages { group_id } => {
                match self.store.get_group_messages(&group_id) {
                     Ok(messages) => {
                         let decrypted_messages = messages.into_iter().map(|node| {
                             let content = self.try_decrypt(&node);
                             (node, content)
                         }).collect();
                         let _ = self.event_tx.send(AppEvent::GroupMessagesFetched(decrypted_messages));
                     }
                     Err(e) => eprintln!("Failed to fetch group messages: {:?}", e),
                }
            }
            AppCmd::ReportContent { target_id, reason, details } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot report content: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Report(dag::ReportPayload { target_id, reason, details });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };

                match dag::DagNode::new(
                    "report:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                         println!("Created report node: {}", node.id);
                         let _ = self.store.put_node(&node);
                         let _ = self.store.update_head(&author_hex, &node.id);
                         let topic = gossipsub::IdentTopic::new("blocks");
                         let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                         let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                         self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create report node: {:?}", e),
                }
            }
            AppCmd::FetchReports => {
                match self.store.get_reports() {
                    Ok(reports) => {
                        let _ = self.event_tx.send(AppEvent::ReportsFetched(reports));
                    }
                    Err(e) => eprintln!("Failed to fetch reports: {:?}", e),
                }
            }
            AppCmd::UploadFile { name, mime_type, data } => {
                if !self.is_caller_verified() {
                    eprintln!("Cannot upload file: User is not verified.");
                    return;
                }
                
                // 1. Create Blob (File Content)
                let blob_payload = dag::DagPayload::Blob(dag::BlobPayload { 
                    mime_type: mime_type.clone(), 
                    data: general_purpose::STANDARD.encode(&data) 
                });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                // Get head for blob (using empty prev for blobs to avoid linearizing content updates if not needed? 
                // Or just use current head. Let's use current head to keep chain.)
                let mut prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    _ => vec![],
                };

                let blob_node = match dag::DagNode::new(
                    "blob:v1".to_string(),
                    blob_payload,
                    prev.clone(), // Use same head
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        let _ = self.store.put_node(&node);
                        let _ = self.store.update_head(&author_hex, &node.id);
                        let topic = gossipsub::IdentTopic::new("blocks");
                        let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                        self.replicate_block(&node);
                        // We don't necessarily emit BlockReceived for the raw blob itself to the UI, 
                        // unless we want to.
                        node
                    },
                    Err(e) => {
                        eprintln!("Failed to create blob node: {:?}", e);
                        return;
                    }
                };

                // Update prev to be the blob we just created
                prev = vec![blob_node.id.clone()];

                // 2. Create File (Metadata)
                let file_payload = dag::DagPayload::File(dag::FilePayload {
                    name,
                    size: data.len() as u64,
                    mime_type,
                    blob_cid: blob_node.id.clone(),
                });

                match dag::DagNode::new(
                    "file:v1".to_string(),
                    file_payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created file node: {}", node.id);
                         let _ = self.store.put_node(&node);
                         let _ = self.store.update_head(&author_hex, &node.id);
                         let topic = gossipsub::IdentTopic::new("blocks");
                         let _ = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes());
                         
                         // Notify UI
                         let _ = self.event_tx.send(AppEvent::BlockReceived(node.clone()));
                         let _ = self.event_tx.send(AppEvent::FileUploaded(node.clone()));
                         
                         self.replicate_block(&node);
                    }
                    Err(e) => eprintln!("Failed to create file node: {:?}", e),
                }
            }
            AppCmd::FetchMyFiles => {
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                match self.store.get_my_files(&author_hex) {
                     Ok(files) => {
                         let _ = self.event_tx.send(AppEvent::MyFilesFetched(files));
                     }
                     Err(e) => eprintln!("Failed to fetch files: {:?}", e),
                }
            }
            AppCmd::PostComment { parent_id, content } => {
                 if !self.is_caller_verified() {
                     eprintln!("Cannot post comment: User is not verified.");
                     return;
                 }
                 let payload = DagPayload::Comment(dag::CommentPayload {
                     parent_id,
                     content,
                     attachments: vec![],
                 });
                 
                 let author_pubkey = self.keypair.public();
                 let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                 
                 let prev = match self.store.get_head(&author_hex) {
                     Ok(Some(cid)) => vec![cid],
                     Ok(None) => vec![],
                     Err(e) => {
                         eprintln!("Failed to get head: {:?}", e);
                         vec![]
                     }
                 };

                 match dag::DagNode::new(
                     "comment:v1".to_string(),
                     payload,
                     prev,
                     &self.keypair,
                     0
                 ) {
                     Ok(node) => {
                         println!("Created comment node: {}", node.id);
                         if let Err(e) = self.store.put_node(&node) {
                             eprintln!("Failed to store comment node: {:?}", e);
                             return;
                         }
                         if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                             eprintln!("Failed to update head: {:?}", e);
                         }
                         let topic = gossipsub::IdentTopic::new("blocks");
                         if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                             eprintln!("Failed to publish comment CID: {:?}", e);
                         }
                         
                         self.replicate_block(&node);
                     }
                     Err(e) => eprintln!("Failed to create comment node: {:?}", e),
                 }
            }
            AppCmd::FetchComments { parent_id } => {
                match self.store.get_comments(&parent_id) {
                    Ok(comments) => {
                        let _ = self.event_tx.send(AppEvent::CommentsFetched { parent_id, comments });
                    }
                    Err(e) => eprintln!("Failed to fetch comments: {:?}", e),
                }
            }
            AppCmd::LikePost { target_id, remove } => {
                 if !self.is_caller_verified() {
                    eprintln!("Cannot like post: User is not verified.");
                    return;
                }
                let payload = dag::DagPayload::Like(dag::LikePayload { target_id: target_id.clone(), remove });
                
                let author_pubkey = self.keypair.public();
                let author_hex = libp2p::PeerId::from_public_key(&author_pubkey).to_string();
                
                let prev = match self.store.get_head(&author_hex) {
                    Ok(Some(cid)) => vec![cid],
                    Ok(None) => vec![],
                    Err(e) => {
                        eprintln!("Failed to get head: {:?}", e);
                        vec![]
                    }
                };

                match dag::DagNode::new(
                    "like:v1".to_string(),
                    payload,
                    prev,
                    &self.keypair,
                    0
                ) {
                    Ok(node) => {
                        println!("Created like node: {}", node.id);
                        if let Err(e) = self.store.put_node(&node) {
                            eprintln!("Failed to store like node: {:?}", e);
                            return;
                        }
                        if let Err(e) = self.store.update_head(&author_hex, &node.id) {
                            eprintln!("Failed to update head: {:?}", e);
                        }
                        let topic = gossipsub::IdentTopic::new("blocks");
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(topic, node.id.as_bytes()) {
                            eprintln!("Failed to publish like CID: {:?}", e);
                        }
                        self.replicate_block(&node);
                        
                        // Immediately fetch updated likes to update UI
                        let my_pubkey = self.local_peer_id().to_string();
                        if let Ok((count, is_liked)) = self.store.get_likes_for_target(&target_id, &my_pubkey) {
                             let _ = self.event_tx.send(AppEvent::LikesFetched { target_id, count, is_liked_by_me: is_liked });
                        }
                    }
                    Err(e) => eprintln!("Failed to create like node: {:?}", e),
                }
            }
            AppCmd::FetchLikes { target_id } => {
                let my_pubkey = self.local_peer_id().to_string();
                match self.store.get_likes_for_target(&target_id, &my_pubkey) {
                    Ok((count, is_liked)) => {
                        let _ = self.event_tx.send(AppEvent::LikesFetched { target_id, count, is_liked_by_me: is_liked });
                    }
                    Err(e) => eprintln!("Failed to fetch likes: {:?}", e),
                }
            }
            AppCmd::FetchMinistries => {
                match self.store.get_active_ministries() {
                     Ok(m) => {
                         let _ = self.event_tx.send(AppEvent::MinistriesFetched(m));
                     }
                     Err(e) => eprintln!("Failed to fetch ministries: {:?}", e),
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
                    request_response::Event::Message { peer: _peer, message } => {
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
                                
                                // Check if this was a search query (key starts with search:term:)
                                if url.starts_with("search:term:") {
                                    let query = url.trim_start_matches("search:term:").to_string();
                                    println!("Sending LocalSearch request for query: {}", query);
                                    for peer in providers {
                                        let _request_id = self.swarm.behaviour_mut().request_response.send_request(&peer, BlockRequest::LocalSearch(query.clone()));
                                        // We don't necessarily need to track this in pending_requests for a block CID, 
                                        // but we can track it to handle errors if we want.
                                        // But BlockResponse::SearchResults processing doesn't rely on pending_requests map for CID.
                                    }
                                } else {
                                    // Normal content fetch
                                    for peer in providers {
                                        let request_id = self.swarm.behaviour_mut().request_response.send_request(&peer, BlockRequest::Fetch(url.clone()));
                                        self.pending_requests.insert(request_id, url.clone());
                                    }
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
                let topic = message.topic.as_str();
                if topic.starts_with("geohash:") {
                    // Presence message
                    let source = message.source;
                    if let Some(peer_id) = source {
                        let peer_str = peer_id.to_string();
                        // If it's not me, emit discovery
                        if peer_id != *self.swarm.local_peer_id() {
                             println!("Discovered peer via presence: {}", peer_str);
                             // We can try to dial them to ensure connection
                             // self.swarm.dial(peer_id); // Might fail if no address? But Gossipsub implies connection or relay.
                             let _ = self.event_tx.send(AppEvent::PeerDiscovered(peer_str));
                        }
                    }
                    return;
                }

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
            photo: None,
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
            photo: None,
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
            photo: None,
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
            photo: None,
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
            photo: None,
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

    #[tokio::test]
    async fn test_dht_search_integration() {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let store = Store::new_in_memory().unwrap();
        
        let mut backend = Backend::new(store, cmd_rx, event_tx, Some(Keypair::generate_ed25519())).await.unwrap();
        
        tokio::spawn(async move {
            backend.run().await;
        });

        // 1. Publish content with tags (this simulates becoming a provider for the tag)
        // We need to be verified first
        cmd_tx.send(AppCmd::PublishProfile {
            name: "Founder".to_string(),
            bio: "First".to_string(),
            photo: None,
        }).unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;

        cmd_tx.send(AppCmd::PublishWebPage {
            url: "sp://cat.super/home".to_string(),
            title: "Cat Page".to_string(),
            content: "<h1>Cats</h1>".to_string(),
            description: "A page about cats".to_string(),
            tags: vec!["cat".to_string()],
        }).unwrap();

        // Wait for publishing to settle (block received, etc.)
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 2. Search for "cat"
        // In a real DHT scenario with multiple nodes, one node would search and find the other.
        // Here we are testing the backend logic to initiate the search correctly.
        // Since we are running single node in this test, we won't find *other* providers,
        // but we can verify that the local search works and the code doesn't crash when triggering DHT search.
        // To properly test interacting with another node via DHT in a unit test is complex due to swarm setup.
        // However, we corrected the request type in the code above.
        
        // Let's at least trigger the local search path which we also touched.
        cmd_tx.send(AppCmd::SearchWeb {
            query: "cat".to_string(),
        }).unwrap();

        // 3. Verify Local Results
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(2) {
                panic!("Timed out waiting for WebSearchResults");
            }
            if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx.recv()).await {
                if let AppEvent::WebSearchResults(nodes) = event {
                    assert!(!nodes.is_empty());
                    if let dag::DagPayload::Web(web) = &nodes[0].payload {
                        assert_eq!(web.title, "Cat Page");
                    }
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn test_social_workflow() {
        
        use crate::backend::dag::DagPayload;

        let (cmd_tx_a, _cmd_rx_a) = mpsc::unbounded_channel();
        let (event_tx_a, mut event_rx_a) = mpsc::unbounded_channel();
        let store_a = Store::new_in_memory().unwrap();
        let mut backend_a = Backend::new(store_a, _cmd_rx_a, event_tx_a, Some(Keypair::generate_ed25519())).await.unwrap();
        
        let (cmd_tx_b, cmd_rx_b) = mpsc::unbounded_channel();
        let (event_tx_b, mut event_rx_b) = mpsc::unbounded_channel();
        let store_b = Store::new_in_memory().unwrap();
        let mut backend_b = Backend::new(store_b, cmd_rx_b, event_tx_b, Some(Keypair::generate_ed25519())).await.unwrap();

        let peer_id_a = backend_a.local_peer_id().to_string();
        let peer_id_b = backend_b.local_peer_id().to_string();

        tokio::spawn(async move {
            backend_a.run().await;
        });

        // Wait for A to start listening
        let addr_a = match tokio::time::timeout(Duration::from_secs(5), event_rx_a.recv()).await {
            Ok(Some(AppEvent::Listening(addr))) => addr,
            _ => panic!("Failed to get listener address"),
        };
        // Fix address for local dialing
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

        // A publishes profile
        cmd_tx_a.send(AppCmd::PublishProfile {
             name: "Alice".to_string(),
             bio: "A".to_string(),
             photo: None,
        }).unwrap();
        
        // B publishes profile
        cmd_tx_b.send(AppCmd::PublishProfile {
             name: "Bob".to_string(),
             bio: "B".to_string(),
             photo: None,
        }).unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        // B Follows A
        cmd_tx_b.send(AppCmd::FollowUser {
            target: peer_id_a.clone(),
            follow: true,
        }).unwrap();
        
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Verify B is following A
        cmd_tx_b.send(AppCmd::FetchFollowing { target: peer_id_b.clone() });
        
        let mut following_verified = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                 if let AppEvent::FollowingFetched(following) = event {
                     if following.contains(&peer_id_a) {
                         following_verified = true;
                         break;
                     }
                 }
             }
        }
        assert!(following_verified, "B failed to follow A");

        // A publishes a Post
        cmd_tx_a.send(AppCmd::PublishPost {
            content: "Hello from Alice".to_string(),
            attachments: vec![],
            geohash: None,
        }).unwrap();
        println!("A published post");

        tokio::time::sleep(Duration::from_secs(2)).await;

        // B Fetches Following Posts
        cmd_tx_b.send(AppCmd::FetchFollowingPosts);
        println!("B fetching following posts");

        let mut post_found = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                 if let AppEvent::FollowingPostsFetched(posts) = event {
                     println!("B received following posts: {}", posts.len());
                     for p in posts {
                         if let DagPayload::Post(payload) = p.payload {
                             if payload.content == "Hello from Alice" {
                                 post_found = true;
                                 break;
                             }
                         }
                     }
                     if post_found { break; }
                 }
             }
        }
        assert!(post_found, "B failed to see A's post in following feed");

        // A publishes a Story
        cmd_tx_a.send(AppCmd::PublishStory {
            media_cid: "fake_cid".to_string(),
            caption: "Story time".to_string(),
            geohash: None,
        }).unwrap();
        println!("A published story");
        
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // B Fetches Stories
        cmd_tx_b.send(AppCmd::FetchStories);
        println!("B fetching stories");
        
        let mut story_found = false;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
             if let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(100), event_rx_b.recv()).await {
                 if let AppEvent::StoriesFetched(stories) = event {
                     println!("B received stories: {}", stories.len());
                     for s in stories {
                         if let DagPayload::Story(payload) = s.payload {
                             if payload.caption == "Story time" {
                                 story_found = true;
                                 break;
                             }
                         }
                     }
                     if story_found { break; }
                 }
             }
        }
        assert!(story_found, "B failed to see A's story");
    }
