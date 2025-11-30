Protocol Specification

Overview: Rules for P2P superapp with equal users, Proof-of-Humanity, tokens without blockchain, content-addressed DAG with dynamic replication.

Node Identity
HumanID: 256-bit unique identifier
PoHSignatures: Array of DAG attestations
PublicKey: Ed25519
Identity valid with threshold signatures (e.g., 5)

Networking Messages
HELLO - HumanID, PublicKey, PoHSignatures
CHUNK_REQUEST - ChunkHash, RequestedPieces
CHUNK_RESPONSE - ChunkHash, DataPieces, Proof
TOKEN_MINT - HumanID, Amount, Signature
TOKEN_BURN - HumanID, Amount, Signature
TOKEN_RECEIVE - HumanID, Amount, Signature
DAG_INSERT - NodeID, ParentHash, ContentHash, Signature
DAG_QUERY - ContentHash
DAG_RESPONSE - NodeID, Content, Proof
PEER_LIST - List of HumanID, Address
SWARM_JOIN - SwarmID, HumanID
SWARM_LEAVE - SwarmID, HumanID

DAG Operations
Node Structure:
NodeID, ParentHashes, ContentHash, Timestamp, Author, Signature
Operations: INSERT, QUERY, VERIFY, MERGE

Chunk Storage & Replication
Chunk Structure: ChunkHash, Data, ErasurePieces, ReplicationTargets
Replication Protocol: track online peers, volunteer when replicas low, store locally, verify with Merkle proofs, periodic rebalancing

Token Operations
Minting: TOKEN_MINT with HumanID, Amount, Signature
Burning: TOKEN_BURN with HumanID, Amount, RecipientID, Signature
Verification: Balance = Σ(MINT) - Σ(BURN)

Proof-of-Humanity Flow
Verification Steps: request PoH, challenge, existing nodes sign, threshold reached
Attestation Storage: PoHNode with VerifiedHumanID, SignerHumanID, Timestamp, Signature

Security Rules

End-to-end encryption with session keys

DAG signatures prevent tampering

Chunk Merkle proofs ensure correct replication

PoH prevents Sybil nodes

Replay protection via timestamps and nonces

