Networking Framework Goals
Robust local meshes (mDNS)
Resilient global discovery (Kademlia)
Efficient pubsub (Gossipsub)
On-demand transfer (Request/Response)
NAT traversal (holepunch/relay)

Core Components
Transport: QUIC for native apps; WebRTC for browsers
Security: Noise or libp2p secio-like encrypted handshakes
Peer Discovery:
- mDNS for LAN peers
- Kademlia DHT for global discovery
- Bootstrap list of ordinary app installs (rotate via releases)
PubSub: gossipsub v1.1 with topic-scoped validators
Request/Response: /superapp/chunk/1.0.0 using streaming framed protocol
Relay/Hole Punching: libp2p relay behavior and WebRTC holepunch

Topologies
1. Local Mesh: mDNS-discovered nodes form tight mesh for local media exchange
2. Fanout Global: gossipsub for public events based on interests
3. Micro-swarms: temporary ephemeral swarms for each manifest

Protocol Namespaces
/superapp/dag/1.0.0 - gossiped DAG events with signature validators
/superapp/pubsub/1.0.0 - gossipsub topics
/superapp/chunk/1.0.0 - fragment fetch with streaming & resumption
/superapp/replicate/1.0.0 - replication negotiation and proof exchange
/superapp/pop/1.0.0 - PoP ceremony coordination

Peer Scoring & Filters
Light-weight peer scoring to penalize invalid events
Validators drop invalid-signed DAG events before re-gossiping

Flow Controls & Congestion
Per-peer rate limits and token-bucket for uploads
For streaming: prefer low RTT peers with sufficient upstream bandwidth

Connection Lifecycle
Minimal keepalive with heartbeats (uptime, storage, geo-tag)
Graceful degradation on poor network: reduce pubsub subscriptions, pause background repairs

Discovery Acceleration
DHT cache: manifest_cid → holder_peerids
Parallel k-bucket queries with exponential backoff on misses

Privacy
Onion-like routing through randomly selected peers (performance tradeoffs)

Implementation
Use existing Rust libp2p crates
Validate all incoming events
Offload CPU-heavy signature checks to worker threads