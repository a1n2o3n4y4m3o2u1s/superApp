Tokenomics

Goals: Fungible tokens for value transfer and resource allocation while maintaining human equality and avoiding blockchain complexity.

Token Primitives
Event types: token:mint, token:burn, token:transfer_claim, token:escrow, token:mint_reward
Tokens recorded as signed DAG events, balances computed by deterministic replay

Core Design Choices
One-person-one-issuer minting: PoP-authenticated nodes authorize periodic minting (UBI-like)
Burn-before-mint transfers: sender-signed burn paired with recipient-signed receipt/mint
Escrow & conditional payments: CRDT-based escrow objects

Typical Flows
UBI/Periodic Minting: Each epoch (daily), PoP validators produce token:mint events awarding X tokens per verified human
Payment for Replication: Uploader publishes token:burn referencing manifest, replicator provides proof-of-storage, mint authority issues token:mint_reward
Marketplace & P2P Trade: Negotiated offline, settled by paired burns and mints in DAG

Anti-Double-Spend Mechanisms
Per-author nonces + deterministic ordering (Lamport ts + pubkey + event id)
Peers reject non-monotonic nonce events
Conflicts mark later event invalid in DAG

Supply & Inflation Policy
Default: steady UBI per verified human per epoch (e.g., 10 tokens/day)
Community governance can adjust UBI rate via PoP voting

Fees & Sinks
Token sinks remove excess tokens: storage fees (burned), donation pools, community treasury
Fees deter spam and pay for resources

Governance & Upgrades
Governance proposals as CRDT objects
PoP members vote (one-person-one-vote or token-weighted)
Upgrades require passed proposal and rollout window

Auditability
Full audit trail in DAG - any node can recompute balances
Tools to scan for invalid events and double-spend attempts