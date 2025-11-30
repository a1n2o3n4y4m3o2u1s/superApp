DAG Schema

Purpose: Define canonical event shapes for validation, indexing, and deterministic history reasoning.

Event Envelope
{
"type": "string",
"id": "CID", // SHA256 of canonical payload
"payload": {...}, // event-specific
"prev": ["CID",...], // DAG parents
"author": "pubkey",
"nonce": u64, // monotonic per-author
"timestamp": "ISO8601",
"sig": "ed25519"
}

Core Event Types & Payloads
profile:v1 - profile update
post:v1 - social post
file:manifest:v1 - file manifest (erasure params, chunk roots, availability)
fragment:announce:v1 - storage announcement
token:mint - minting event
token:burn - burn/spend
pop:attest - PoP attestation
gov:proposal - governance proposal
gov:vote - vote

Parent Linking Rules
Events must include non-empty prev array linking to recent local heads
Use DAG heads per author; merge by including multiple prevs

Canonical Ordering for Conflicts
Deterministic ordering: (LamportTimestamp, author_pubkey_hex, event_id_hex) ascending

Invalidation & Evidence
invalid:event records link to invalidated event with evidence
Keep invalid entries for audit

Indexing
Indices by: author, type, referenced_cid, timestamp range
Availability table for manifests maintained by replication manager

Snapshots & Pruning
Periodic signed CRDT snapshots compact state
Keep raw events for 90 days for audits

Validation Rules
Signature verification for all events
Nonce monotonicity checks for token events
PoP-attestation validation

