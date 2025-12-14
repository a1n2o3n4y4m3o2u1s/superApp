use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
pub use libp2p::identity::{Keypair, PublicKey};
use libp2p::PeerId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagNode {
    pub r#type: String,
    pub id: String, // CID
    pub payload: DagPayload,
    pub prev: Vec<String>, // CIDs of parents
    pub author: String, // PeerID string
    pub public_key: String, // Public key hex
    pub nonce: u64,
    pub timestamp: DateTime<Utc>,
    pub sig: String, // Hex encoded signature
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReputationDetails {
    pub score: u32,
    pub breakdown: ReputationBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReputationBreakdown {
    pub verification: u32,
    pub storage: u32,
    pub content: u32,
    pub governance: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FollowPayload {
    pub target: String,
    pub follow: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "payload")]
pub enum DagPayload {
    #[serde(rename = "profile:v1")]
    Profile(ProfilePayload),
    #[serde(rename = "post:v1")]
    Post(PostPayload),
    #[serde(rename = "proof:v1")]
    Proof(ProofPayload),
    #[serde(rename = "message:v1")]
    Message(MessagePayload),
    #[serde(rename = "group:v1")]
    Group(GroupPayload),
    #[serde(rename = "token:v1")]
    Token(TokenPayload),
    #[serde(rename = "web:v1")]
    Web(WebPayload),
    #[serde(rename = "name:v1")]
    Name(NamePayload),
    #[serde(rename = "blob:v1")]
    Blob(BlobPayload),
    #[serde(rename = "listing:v1")]
    Listing(ListingPayload),
    #[serde(rename = "contract:v1")]
    Contract(ContractPayload),
    #[serde(rename = "contract_call:v1")]
    ContractCall(ContractCallPayload),
    #[serde(rename = "proposal:v1")]
    Proposal(ProposalPayload),
    #[serde(rename = "vote:v1")]
    Vote(VotePayload),
    #[serde(rename = "candidacy:v1")]
    Candidacy(CandidacyPayload),
    #[serde(rename = "candidacy_vote:v1")]
    CandidacyVote(CandidacyVotePayload),
    #[serde(rename = "report:v1")]
    Report(ReportPayload),
    #[serde(rename = "file:v1")]
    File(FilePayload),
    #[serde(rename = "recall:v1")]
    Recall(RecallPayload),
    #[serde(rename = "recall_vote:v1")]
    RecallVote(RecallVotePayload),
    #[serde(rename = "oversight_case:v1")]
    OversightCase(OversightCasePayload),
    #[serde(rename = "jury_vote:v1")]
    JuryVote(JuryVotePayload),
    #[serde(rename = "comment:v1")]
    Comment(CommentPayload),
    #[serde(rename = "like:v1")]
    Like(LikePayload),
    #[serde(rename = "story:v1")]
    Story(StoryPayload),
    #[serde(rename = "follow:v1")]
    Follow(FollowPayload),
    // Education System
    #[serde(rename = "course:v1")]
    Course(CoursePayload),
    #[serde(rename = "exam:v1")]
    Exam(ExamPayload),
    #[serde(rename = "exam_submission:v1")]
    ExamSubmission(ExamSubmissionPayload),
    #[serde(rename = "certification:v1")]
    Certification(CertificationPayload),
    // Verification Application System
    #[serde(rename = "application:v1")]
    Application(ApplicationPayload),
    #[serde(rename = "application_vote:v1")]
    ApplicationVote(ApplicationVotePayload),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryPayload {
    pub media_cid: String,
    #[serde(default)]
    pub caption: String, 
    #[serde(default)]
    pub geohash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommentPayload {
    pub parent_id: String, // CID of the post or comment being replied to
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LikePayload {
    pub target_id: String, // CID of the post/comment being liked
    pub remove: bool, // true if this is an "unlike" action (toggle off)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OversightCasePayload {
    pub case_id: String,
    pub report_id: String,
    pub jury_members: Vec<String>, // List of public keys (hex)
    pub status: String, // "Open", "Closed"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JuryVotePayload {
    pub case_id: String,
    pub vote: String, // "Uphold", "Dismiss"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GroupPayload {
    pub name: String,
    pub members: Vec<String>, // List of Peer IDs (hex pubkeys)
    pub owner: String, // Founder of the group
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NamePayload {
    pub name: String,
    pub target: String, // e.g. pubkey hex or CID
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilePayload {
    pub name: String,
    pub bio: String,
    pub founder_id: Option<u32>,
    pub encryption_pubkey: Option<String>, // Hex encoded X25519 public key
    pub photo: Option<String>, // CID of profile photo blob
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PostPayload {
    pub content: String,
    #[serde(default)]
    pub attachments: Vec<String>, // CIDs of blobs
    #[serde(default)]
    pub geohash: Option<String>, // Optional location tag
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlobPayload {
    pub mime_type: String,
    pub data: String, // Base64 encoded data
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ListingPayload {
    pub title: String,
    pub description: String,
    pub price: u64, // SUPER tokens
    pub image_cid: Option<String>,
    pub category: Option<String>,
    pub status: ListingStatus,
    pub ref_cid: Option<String>, // Reference to the original listing CID if this is an update
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ContractStatus {
    #[default]
    Pending,   // Awaiting counterparty acceptance
    Active,    // Both parties agreed
    Completed, // Fulfilled
    Rejected,  // Counterparty rejected
    Cancelled, // Creator cancelled
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractPayload {
    pub code: String, // Source code or WASM hex
    pub init_params: String, // JSON string
    pub status: ContractStatus, // Lifecycle status
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractCallPayload {
    pub contract_id: String, // CID of the contract node
    pub method: String,
    pub params: String, // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProposalPayload {
    pub title: String,
    pub description: String,
    pub r#type: ProposalType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    Standard,
    Constitutional,
    Emergency,
    SetTax(u8), // Tax rate in percent (0-100)
    DefineMinistries(Vec<String>), // List of ministry names
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VotePayload {
    pub proposal_id: String,
    pub vote: VoteType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteType {
    Yes,
    No,
    Abstain,
    PetitionSignature,
}

/// Ministry identifier (e.g., "VerificationAndIdentity")
pub type Ministry = String;

/// A verified user declaring candidacy for a ministry position
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidacyPayload {
    pub ministry: Ministry,
    pub platform: String, // Candidate's platform/statement
}

/// A vote for a specific candidate (identified by their candidacy node CID)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidacyVotePayload {
    pub candidacy_id: String, // CID of the candidacy node
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReportPayload {
    pub target_id: String, // CID of the reported content
    pub reason: String, // e.g. "Spam", "Illegal", "Harassment"
    pub details: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilePayload {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub blob_cid: String, // CID of the BlobPayload
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProofPayload {
    pub target_pubkey: String, // Hex encoded public key of the person being verified
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessagePayload {
    pub recipient: String, // PeerId string
    pub ciphertext: String, // Hex encoded encrypted content
    pub nonce: String, // Hex encoded nonce
    pub ephemeral_pubkey: String, // Hex encoded ephemeral public key of sender
    pub group_id: Option<String>, // Optional: CID of the group if this is a group message
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenPayload {
    pub action: TokenAction,
    pub amount: u64,
    pub target: Option<String>, // Recipient pubkey or other target identifier
    pub memo: Option<String>,
    pub ref_cid: Option<String>, // Reference to a previous event (e.g., the burn event being claimed)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenAction {
    Mint,
    Burn,
    TransferClaim,
    Escrow,
    MintReward,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebPayload {
    pub url: String, // e.g. sp://alice.super/home
    pub title: String,
    pub content: String, // HTML/Markdown content
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecallPayload {
    pub target_official: String, // Hex pubkey of the official to recall
    pub ministry: Ministry,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecallVotePayload {
    pub recall_id: String, // CID of the recall node
    pub vote: bool, // true = remove, false = keep
}

// ============ EDUCATION SYSTEM ============

/// A course curriculum that anyone can publish
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CoursePayload {
    pub title: String,
    pub description: String,
    pub content: String, // Markdown/HTML content
    pub category: CourseCategory,
    pub exam_id: Option<String>, // Optional CID of the associated exam
    pub prerequisites: Vec<String>, // CIDs of prerequisite certifications
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CourseCategory {
    CivicLiteracy,
    GovernanceRoles,
    TechnicalSkills,
    TradeQualifications,
    ModerationJury,
    Custom(String),
}

/// An exam that can be taken to earn certification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExamPayload {
    pub title: String,
    pub course_id: Option<String>, // Associated course CID
    pub questions: Vec<ExamQuestion>,
    pub passing_score: u8, // Minimum percentage to pass (0-100)
    pub certification_type: String, // The cert type this exam grants
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExamQuestion {
    pub question: String,
    pub options: Vec<String>,
    pub correct_answer_hash: String, // SHA256 of correct option index (for verification)
}

/// User's submission for an exam
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExamSubmissionPayload {
    pub exam_id: String, // CID of the exam
    pub answers: Vec<usize>, // Indices of selected options
    pub score: u8, // Calculated score percentage
    pub passed: bool,
}

/// A certification issued to a user by peer consensus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CertificationPayload {
    pub recipient: String, // Hex pubkey of the certified user
    pub certification_type: String, // e.g., "CivicLiteracy", "ModeratorEligible"
    pub exam_id: Option<String>, // CID of the exam passed
    pub issuer_signatures: Vec<String>, // Signatures from certified peers
    pub issued_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Join application submitted by new users seeking verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationPayload {
    pub name: String,
    pub bio: String,
    pub photo_cid: Option<String>, // CID of uploaded selfie/photo
}

/// Vote on a join application by a verified user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApplicationVotePayload {
    pub application_id: String, // CID of the application node
    pub approve: bool, // true = approve, false = reject
}

impl DagNode {
    pub fn new(
        r#type: String,
        payload: DagPayload,
        prev: Vec<String>,
        keypair: &Keypair,
        nonce: u64,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let timestamp = Utc::now();
    let author_pubkey = keypair.public();
    let peer_id = PeerId::from_public_key(&author_pubkey);
    let author_str = peer_id.to_string();
    let pubkey_hex = hex::encode(author_pubkey.encode_protobuf());

    let mut node = Self {
        r#type,
        id: String::new(), // Placeholder
        payload,
        prev,
        author: author_str,
        public_key: pubkey_hex,
        nonce,
            timestamp,
            sig: String::new(), // Placeholder
        };

        // Calculate CID (ID)
        let cid = node.calculate_cid()?;
        node.id = cid;

        // Sign
        let sig = node.sign(keypair)?;
        node.sig = sig;

        Ok(node)
    }

    pub fn calculate_cid(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Canonical serialization for hashing:
        // We need to hash the fields *except* id and sig.
        // A simple way is to create a struct with just those fields, or manually serialize.
        // For now, let's serialize the payload and other metadata.
        // NOTE: In a real production system, we'd use a more robust canonical serialization (e.g. IPLD/CBOR).
        // Here we'll use JSON of a specific structure for simplicity.
        
        #[derive(Serialize)]
    struct CanonicalView<'a> {
        r#type: &'a str,
        payload: &'a DagPayload,
        prev: &'a [String],
        author: &'a str,
        public_key: &'a str,
        nonce: u64,
        timestamp: DateTime<Utc>,
    }

    let view = CanonicalView {
        r#type: &self.r#type,
        payload: &self.payload,
        prev: &self.prev,
        author: &self.author,
        public_key: &self.public_key,
        nonce: self.nonce,
        timestamp: self.timestamp,
    };

        let json = serde_json::to_string(&view)?;
        let mut hasher = Sha256::new();
        hasher.update(json);
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    pub fn sign(&self, keypair: &Keypair) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // We sign the CID (which represents the content)
        let msg = self.id.as_bytes();
        let sig = keypair.sign(msg)?;
        Ok(hex::encode(sig))
    }

    pub fn verify(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Re-calculate CID and check if it matches self.id
        let calculated_cid = self.calculate_cid()?;
        if calculated_cid != self.id {
            return Ok(false);
        }

        // 2. Decode public key from field
    let pubkey_bytes = hex::decode(&self.public_key)?;
    let pubkey = PublicKey::try_decode_protobuf(&pubkey_bytes)?;

    // Verify that the author PeerID matches this public key
    let derived_peer_id = PeerId::from_public_key(&pubkey);
    if derived_peer_id.to_string() != self.author {
         return Ok(false);
    }

    // 3. Verify signature against CID
    let sig_bytes = hex::decode(&self.sig)?;
    let msg = self.id.as_bytes();
    
    Ok(pubkey.verify(msg, &sig_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dag_node_creation_and_verification() {
        let keypair = Keypair::generate_ed25519();
        let payload = DagPayload::Post(PostPayload {
            content: "Hello World".to_string(),
            attachments: vec![],
            geohash: None,
        });

        let node = DagNode::new(
            "post:v1".to_string(),
            payload,
            vec!["parent_cid".to_string()],
            &keypair,
            1,
        ).expect("Failed to create node");

        assert!(node.verify().expect("Verification failed"));
    }
}
