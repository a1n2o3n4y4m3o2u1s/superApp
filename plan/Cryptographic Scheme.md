Cryptographic Scheme

Goals: Strong signatures for events, secure encryption for private content, non-transferable uniqueness attestations, lightweight fragment integrity proofs.

Key Types & Use-Cases

Identity Keypair (Ed25519) - primary signing for DAG events

X25519 Keypair - ephemeral DH for session keys

Symmetric Keys (ChaCha20-Poly1305) - encrypt private fragments & messages

Merkle roots - chunk manifests and fragment verification

Algorithms & Libraries

Signatures: Ed25519 (ed25519-dalek)

KEX: X25519 (x25519-dalek)

AEAD: ChaCha20-Poly1305

Hashing: SHA-256

Erasure coding: Reed-Solomon

Event Signing & Canonicalization
Canonical serialization → payload_hash = SHA256(serialized_payload) → build event header → sign header with Ed25519 → produce event with signature
Verification: check signature, payload_hash, prev links

Fragment Integrity Proofs
Merkle tree over fragment bytes
manifest.fragment[].fragment_cid = SHA256(fragment_bytes)
manifest.chunk_root = merkle_root
Serve fragments with Merkle proof path

Token Events Anti-Replay
Every token event includes nonce (monotonic per-author)
Peers reject duplicate or lower nonce events

Session & Transport Encryption
libp2p secure transports (Noise or TLS)
E2E symmetric keys per peer pair (X25519 DH)

Key Rotation & Recovery
Key rotation event: new_pubkey linked to old_pubkey signed by old key
Account recovery: encrypted key backup with passphrase, social recovery via multi-sig