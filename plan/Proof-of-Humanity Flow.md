Proof-of-Humanity Flow

Objectives: Sybil resistance (one-person-one-node), privacy preservation, practical on mobile, resilient to adversarial behavior.

PoP Primitives
Ceremony: ephemeral small-group session (video/audio/text challenge)
Attestation: signed pop:attest linking subject pubkey → ceremony evidence CID
Uniqueness Token: non-transferable signed attestation marking verified pubkey

Ceremony Types (Tiered)

Automated micro-ceremony - short challenge with liveness check + ephemeral token

Peer-attest ceremony - 3-5 participants vouch for each other

Verifier-assisted ceremony - trusted community verifiers

Flow: Peer-Attest Example

Group formation: PoP scheduler gathers n candidate nodes randomly

Challenge: synchronized challenge (pose, phrase), record proof

Voting: participants review responses and sign attestations

Attestation publication: verifiers publish pop:attest referencing evidence CID

Uniqueness threshold: t independent attestations = verified status

Preventing Collusion & Attacks
Randomized grouping with unpredictable composition
Diversity constraints: prefer geographic/ISP-diverse groups
Rate limits for verifier attestations
Non-replayable challenge responses

Privacy-Preserving Variants
Selective disclosure: ZK proofs without raw biometric data
Ephemeral witnesses: store only media hash in DAG, raw media encrypted locally

Attestation Verification
Peers verify pop:attest signatures and evidence CID existence
Evidence expiry window prevents replay

Uniqueness Token Lifecycle
Issue: after threshold attestations, specifies epoch & expiry
Renewal: annual expiry with lightweight ceremony
Revocation: pop:revoke event with fraud proof, requires verifier quorum

Handling False Attestations
pop:dispute events with evidence
Review by rotating PoP jury
Possible token invalidation