Mobile App Architecture

Purpose: Dioxus UI connecting to embedded P2P backend, storage, and background services. Optimize for battery, bandwidth, and privacy.

High-Level Layers

UI Layer (Dioxus) - cross-platform components, async tasks to backend

IPC/Bridge - JSON-RPC or Tokio channels between UI and backend

P2P Backend (Rust) - Networking, DAG store, CRDT engine, Token engine, Replication manager, PoP manager, Storage manager

Local Persistence - Lightweight DB (sled/sqlite) for DAG events, CRDT snapshots, manifests, fragments index, token event cache

Power & Connectivity Manager - Controls sync frequency based on battery, network type, privacy settings

Security & Key Store - Platform key store holding identity and encryption keys

Process Flows
App Startup: UI spawns backend, loads keys/DB/CRDT, initializes libp2p swarm
Upload Video: UI calls upload_file → backend chunker + erasure encoder → manifest published → replication begins
Token Send: UI calls send_tokens → signed token:burn event → published to DAG pubsub → local balance updated

Background Sync Strategy
Active mode (foreground): low latency pubsub, aggressive repair, prefetch
Background mode (opt-in): periodic heartbeat, repair every 15-30 minutes, prioritized prefetch
Standby (no permission): only respond to direct requests

Resource Controls
Max storage quota (user adjustable)
Network throttles per interface
Fragment caching TTL and LRU eviction

Battery & Privacy
Push notifications only if user opts in
Wi-Fi-only large transfers
Keep biometric evidence local