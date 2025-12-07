# 🌌 P2P SuperApp: The Post-Cloud OS

<p align="center">
  <strong>Complete Digital Sovereignty. Zero Servers. 100% Peer-to-Peer.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75+-black?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Stack-Libp2p%20%7C%20Dioxus%20%7C%20Tokio-blue?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Architecture-DAG%20%2B%20Gossipsub-purple?style=for-the-badge" />
</p>

---

## 📜 Overview

**P2P SuperApp** is a monolithic decentralized application that replaces the entire cloud stack (Social, Commerce, Governance, Messaging) with a single local-first binary. It runs no central servers; every feature relies on a custom **Directed Acyclic Graph (DAG)** synced via **Libp2p Gossipsub**.

This document details the **7-Page Application Structure** and the actual implemented technology behind it, based on the current codebase.

---

## 📱 The 7-Page Architecture

 The application is divided into 7 core experiences, all sharing the same `Keypair` identity and `Store`.

### 1. 🏠 Home (Social Core)
*Implemented in `src/components/home_page.rs`*

A fully decentralized social feed similar to Instagram/Twitter.
- **Protocol**: Nodes of type `post:v1`, `story:v1`, `comment:v1`, `like:v1`.
- **Features**:
    - **Global vs. Following Feed**: Users can switch between a firehose of all network content (`AppCmd::FetchPosts`) and a curated graph of followed peers (`AppCmd::FetchFollowingPosts`).
    - **Encrypted Stories**: 24-hour ephemeral media. Media blobs are encrypted, and the decryption key is discarded after expiration.
    - **Rich Media**: Supports image/video sharing via `BlobPayload`. The app caches blobs locally (`blob_cache`) to prevent re-fetching.
    - **Social Actions**: Like (❤️), Reply (threaded comments), and Repost functionality.

### 2. 📍 Local (Geohash Discovery)
*Implemented in `src/components/geohash_page.rs`*

A hyper-local mesh network for physical communities.
- **Protocol**: `post:v1` with optional `geohash` field (e.g., `u4pru`).
- **Features**:
    - **Precision Zoom**: Users select their broadcast radius: Global → Continent → Country → Region → City → Neighborhood.
    - **Auto-Detection**: The app uses IP-based geolocation (`AppCmd::AutoDetectGeohash`) to bootstrap discovery.
    - **Local Feed**: Shows posts *only* relevant to the selected geohash prefix.
    - **Nearby Peers**: Discovers other users on the same LAN (mDNS) or DHT bucket for offline-first coordination.

### 3. 🌐 Web (SuperWeb Browser)
*Implemented in `src/components/browser_page.rs` & `vm.rs`*

A built-in decentralized web browser.
- **Protocol**: `web:v1`, `sp://` URI scheme.
- **Features**:
    - **Serverless Hosting**: Users publish websites directly to the mesh (`AppCmd::PublishWebPage`), with Title, Description, and Tags.
    - **Search Engine**: Two modes: **Web Search** (for `sp://` sites) and **File Search** (for shared blobs). Queries the DHT tags.
    - **WASM Support**: The browser detects `\0asm` magic bytes in content and spins up a sandboxed `WasmRuntime` (`vm.rs`) to execute dynamic apps client-side.
    - **Governance Integration**: Redirects `sp://gov.super` to the native Governance page.

### 4. 🏪 Market (P2P Economy)
*Implemented in `src/components/marketplace_page.rs`*

A trust-minimized marketplace using the native SUPER token.
- **Protocol**: `listing:v1`, `token:v1`.
- **Features**:
    - **Listing Creation**: Title, Description, Price (SUPER), and optional Image.
    - **Order Management**: Sellers can mark items as `Sold` or `Cancelled`.
    - **Direct Purchase**: "Buy Now" triggers an atomic token transfer.
    - **Asset Search**: specialized search filter for market listings.

### 5. 🏛️ Govern (Direct Democracy)
*Implemented in `src/components/governance_page.rs`*

A comprehensive 1-Person-1-Vote constitution engine.
- **Protocol**: `proposal:v1`, `vote:v1`, `candidacy:v1`, `recall:v1`, `oversight_case:v1`.
- **Features**:
    - **Proposal Types**: `Standard`, `Constitutional` (66% threshold), `Emergency` (48h), `SetTax`, `DefineMinistries`.
    - **Elections**: Citizens declare candidacy for Ministries (e.g., "VerificationAndIdentity"); others vote them into power.
    - **Recall System**: Any official can be targeted for Recall. If the `RecallVote` passes (Remove > Keep), they are stripped of power.
    - **Jury Duty**: The system assigns random citizens to `OversightCase` disputes. Jurors vote "Uphold" or "Dismiss" on reported content.

### 6. 💬 Messages (Secure Comms)
*Implemented in `src/components/messaging_page.rs`*

Military-grade encrypted communication.
- **Protocol**: `message:v1`, `group:v1`, `file:v1`.
- **Encryption**: Uses **AES-256-GCM** for payloads and **X25519** for key exchange. Content is opaque to the network.
- **Features**:
    - **Direct Messages**: 1-on-1 encrypted chat.
    - **Group Chats**: Users create named groups (`AppCmd::CreateGroup`). Messages are fanned out to all members.
    - **Secure File Sharing**: Files are encrypted chunk-by-chunk on the client *before* upload. The recipient receives `[FILE:cid:key:nonce:mime:filename]` to decrypt locally.

### 7. 👤 Profile (Sovereign Identity)
*Implemented in `src/components/profile_page.rs`*

The user's command center.
- **Protocol**: `profile:v1`, `proof:v1`.
- **Features**:
    - **Proof-of-Humanity**: Users must be Vouched (`AppCmd::Vouch`) by existing Verified citizens to gain `Verified` status.
    - **Reputation Score**: A breakdown of trust across Verification, Content, Governance, and Storage axes.
    - **UBI Wallet**: Verified users click "Claim UBI" to mint 10 SUPER/day. Includes a visual countdown timer.
    - **Smart Contract Console**: A developer UI to Deploy (`AppCmd::DeployContract`) and Call (`AppCmd::CallContract`) raw logic on the network.

---

## 🛠️ Technical Implementation Details

### Data Layer: The DAG
Defined in `src/backend/dag.rs`.
Every piece of data is a `DagNode` containing:
- `id` (CID: Content ID)
- `prev` (Parent CIDs for causality)
- `author` (PubKey)
- `sig` (Ed25519 Signature)
- `payload` (**25+ Types** implemented: `Post`, `Vote`, `Recall`, `Listing`, `Contract`, etc.)

### Execution Layer: The VM
Defined in `src/backend/vm.rs`.
- **Hybrid State**: Uses `sqlite` for persistent indexing and `HashMap` for hot contract state.
- **WASM Runtime**: Executes untrusted code (Smart Contracts & Web Apps) in a secure sandbox.
- **KV Store**: Contracts manipulate a deterministic Key-Value store (`set`, `delete`).

### Network Layer: Libp2p
Defined in `src/backend/network.rs`.
- **Transport**: TCP + WebSocket + DNS.
- **Discovery**: Kademlia DHT (Global) + mDNS (Local).
- **Gossip**: `gossipsub` with topic validation ensures generic spam is rejected.

---

## 🗺️ Progress & Implementation Matrix

Based on deep code analysis of `src/` and `plan.md`:

### ✅ Fully Implemented
- [x] **Core P2P Stack**: Networking, DAG, Storage, Identity.
- [x] **UI Shell**: 7-Page Navigation System (Desktop).
- [x] **Social**: Feed, Stories (blob-based), Follow Graph, Comments, Likes.
- [x] **Messaging**: E2E Encryption, Group logic, File Sharing (AES-256).
- [x] **Governance**: Proposals (Tax/Ministries), Voting, Candidacy, Recalls, Jury Duty.
- [x] **Marketplace**: Listings, Buying, Status Updates.
- [x] **Browser**: URL bar, Content Rendering, Publishing, Search.
- [x] **Identity**: Profiles, Vouching, Reputation Calculation.
- [x] **Economy**: UBI Timer, Minting, Transfers, Balance Tracking.
- [x] **Smart Contracts**: Deployment and Execution console.

### 🔄 In Progress / Partial
- [ ] **Mobile Layouts**: `src/components/*.rs` are optimized for Desktop.
- [ ] **Advanced Reputation Math**: Basic counters exist; EigenTrust algorithm pending.

### 🔮 Future
- [ ] **WebRTC**: Audio/Video calls (placeholder in plan).
- [ ] **iOS/Android Ports**: Native compilation targets.

---

## 📦 Getting Started

### Prerequisites
- **Rust**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Dioxus CLI**: `cargo install dioxus-cli`
- **System Dependencies**: `openssl`, `pkg-config`

### Running the Node
```bash
# 1. Clone
git clone https://github.com/your-username/superApp.git
cd superApp

# 2. Run Desktop App (Hot Reload)
dx serve --desktop
```

---

## 📄 License
**Proprietary / Closed Source**. All rights reserved.
Codebase analysis and documentation generated by Antigravity Agent.
