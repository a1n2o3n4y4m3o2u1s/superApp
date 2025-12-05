use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};
use libp2p::identity::{Keypair, PublicKey};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DagNode {
    pub r#type: String,
    pub id: String, // CID
    pub payload: DagPayload,
    pub prev: Vec<String>, // CIDs of parents
    pub author: String, // Public key hex
    pub nonce: u64,
    pub timestamp: DateTime<Utc>,
    pub sig: String, // Hex encoded signature
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum DagPayload {
    #[serde(rename = "profile:v1")]
    Profile(ProfilePayload),
    #[serde(rename = "post:v1")]
    Post(PostPayload),
    #[serde(rename = "proof:v1")]
    Proof(ProofPayload),
    #[serde(rename = "message:v1")]
    Message(MessagePayload),
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
    pub status: ListingStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ListingStatus {
    Active,
    Sold,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractPayload {
    pub code: String, // Source code or WASM hex
    pub init_params: String, // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractCallPayload {
    pub contract_id: String, // CID of the contract node
    pub method: String,
    pub params: String, // JSON string
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
        let author_hex = hex::encode(author_pubkey.encode_protobuf());

        let mut node = Self {
            r#type,
            id: String::new(), // Placeholder
            payload,
            prev,
            author: author_hex,
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
            nonce: u64,
            timestamp: DateTime<Utc>,
        }

        let view = CanonicalView {
            r#type: &self.r#type,
            payload: &self.payload,
            prev: &self.prev,
            author: &self.author,
            nonce: self.nonce,
            timestamp: self.timestamp,
        };

        let json = serde_json::to_string(&view)?;
        let mut hasher = Sha256::new();
        hasher.update(json);
        let result = hasher.finalize();
        Ok(hex::encode(result))
    }

    fn sign(&self, keypair: &Keypair) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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

        // 2. Decode author public key
        let pubkey_bytes = hex::decode(&self.author)?;
        let pubkey = PublicKey::try_decode_protobuf(&pubkey_bytes)?;

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
