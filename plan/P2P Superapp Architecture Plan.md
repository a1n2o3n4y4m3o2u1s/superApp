P2P Superapp Architecture Plan

Fully P2P superapp with social features, profiles, videos, and data. No blockchain, no centralized servers, no special pinning nodes. Everyone equal, proof-of-humanity, tokens/currency, and dynamic replication.

Identity & Proof-of-Humanity Layer
Distributed Proof-of-Humanity (dPoH)

New users verified by X existing users via video verification, biometric liveness detection, challenge/response tasks

Verification signatures stored in DAG

Threshold reached (e.g., 5 signatures) = identity accepted

Unique Token: non-transferable signed attestations in DAG

Data Storage Layer
Dynamic Replication (10 Holders Rule)

Target replication factor: minimum 10 online nodes per chunk

Nodes randomly take responsibility when replicas drop below target

Automatic rebalancing when nodes go offline
Neighborhood-Based Replication: group by interest graph, geography, friend graph
Erasure Coding: split files into k pieces, only m needed to reconstruct

Data Addressing & Versioning
Content-Addressed DAG

Posts, profiles, video chunks hashed

Mutations form version chains (Merkle-DAG)

Enables local caching, deduplication, user-controlled data

Networking Layer
Peer Discovery: DHT (Kademlia-like), multicast DNS for local discovery, optional NAT relays
Swarm Networking: temporary micro-swarms for video streaming
End-to-End Encryption for private content

Tokens + Currency (No Blockchain)
Option A - Minted by Humanity:

Users periodically mint tokens (UBI-style)

Tokens destroyed when spent

Receiver mints new tokens equal to spent amount

No global consensus needed

Option B - Hashgraph-Style Gossip Ledger
Option C - Local Token Pools with CRDTs

Social Media Layer

Profiles as DAG-linked objects

Feeds from following graph

Encrypted P2P messaging

Comments and reactions as DAG nodes

Incentives (Non-Hierarchical)

Contribute storage → token rewards

Stay online → uptime points

Help verify humans → bonus

No mining, no validators, no power compounding

Security Model

Automatic chunk verification (Merkle proofs)

Sybil-resistant identities

End-to-end encrypted communications

No single point of failure

Nodes don't need to trust each other