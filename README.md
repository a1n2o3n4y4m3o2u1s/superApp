# P2P SuperApp

<p align="center">
  <strong>A fully decentralized, peer-to-peer SuperApp combining social media, encrypted messaging, payments, file storage, and a decentralized web.</strong>
</p>

<p align="center">
  Built with Rust 🦀 and <a href="https://dioxuslabs.com">Dioxus 0.7</a>
</p>

---

## 🌟 Overview

P2P SuperApp is an ambitious project to build a comprehensive digital ecosystem that operates **entirely without central servers**. It leverages cutting-edge P2P technologies to create a truly decentralized alternative to traditional social networks, messaging apps, and web browsers—all in a single application.

### Philosophy

- **🚫 No Central Servers** — Everything is peer-to-peer. Your data lives on your device and is replicated across trusted peers.
- **🚫 No Blockchain** — No consensus mechanisms, no mining, no on-chain tokens. Just pure P2P networking.
- **🚫 No Hierarchy** — All users are equal (with special recognition for Founders).
- **🔐 Privacy First** — End-to-end encryption for all private communications.
- **🌍 Location-Aware** — Geohash-based discovery without central tracking.
- **🌐 SuperWeb** — A built-in P2P World Wide Web (`sp://` protocol).

---

## ✨ Features

### 📱 Social Feeds
- **Global Feed** — Twitter-like posts visible to the entire network
- **Local Feed** — Location-based posts filtered by geohash proximity
- **Media Attachments** — Upload and share images directly in posts
- **Blob Storage** — Content-addressable storage for media files

### 💬 Encrypted Messaging
- **End-to-End Encryption** — All private messages use X25519 key exchange with AES-256-GCM
- **Ephemeral Keys** — Each message uses ephemeral key pairs for forward secrecy
- **Peer Discovery** — Connect with anyone using their Peer ID

### 💰 Token Economy
- **SUPER Tokens** — Native token system for the network
- **Universal Basic Income (UBI)** — Verified users can claim daily UBI rewards
- **P2P Transfers** — Send tokens directly between peers (burn-claim mechanism)
- **Escrow Support** — Token escrow for secure transactions

### 🏪 Marketplace
- **P2P Trading** — Create listings and trade items/services for SUPER tokens
- **Direct Contact** — Connect directly with sellers via their profile
- **No Middleman** — All transactions are peer-to-peer

### 🌐 SuperWeb Browser
- **sp:// Protocol** — Browse decentralized websites hosted by peers
- **Name Resolution** — Human-readable domain names (e.g., `sp://alice.super/home`)
- **DHT Discovery** — Content discovery via Kademlia DHT
- **WASM Rendering** — Dynamic pages rendered by WebAssembly modules
- **Search** — Search for published web content by title, description, or tags

### 📜 Smart Contracts
- **Local Execution** — Contracts execute locally for instant results
- **WASM Runtime** — Full WebAssembly support via wasmi
- **KV-Store Contracts** — Simple JSON-based state machine for key-value storage
- **Method Calls** — Call contract methods with JSON parameters

### ✅ Proof-of-Humanity
- **Founder Status** — First 100 users are automatically verified as Founders
- **Peer Vouching** — Verified users can vouch for new users
- **Verification Chain** — Recursive verification through vouching network
- **Gated Access** — Unverified users have restricted access to features

---

## 🏗️ Architecture

### Project Structure

```
superApp/
├── src/
│   ├── main.rs              # Application entry point and routing
│   ├── backend/
│   │   ├── mod.rs           # Core backend logic and command handling (2400+ lines)
│   │   ├── dag.rs           # Directed Acyclic Graph data structures
│   │   ├── store.rs         # SQLite/In-memory storage engine
│   │   ├── network.rs       # libp2p network behavior configuration
│   │   ├── identity.rs      # Keypair management and persistence
│   │   ├── vm.rs            # Smart contract virtual machine
│   │   └── wasm.rs          # WASM runtime for contract execution
│   └── components/
│       ├── mod.rs           # Component exports and AppState
│       ├── home_page.rs     # Global and local social feeds
│       ├── messaging_page.rs # E2E encrypted messaging
│       ├── profile_page.rs  # User profile, wallet, and contracts
│       ├── browser_page.rs  # SuperWeb browser
│       ├── marketplace_page.rs # P2P marketplace
│       ├── geohash_page.rs  # Location-based discovery
│       ├── verification_page.rs # Proof-of-Humanity verification
│       └── nav_bar.rs       # Navigation component
├── assets/                  # Static assets (CSS, images)
├── Cargo.toml              # Rust dependencies
└── Dioxus.toml             # Dioxus configuration
```

### Backend Components

#### DAG (Directed Acyclic Graph) — `dag.rs`

The DAG is the core data structure for all content in the network. Every piece of content is a signed, content-addressed node:

```rust
pub struct DagNode {
    pub id: String,           // Content ID (SHA-256 hash)
    pub r#type: String,       // Node type (e.g., "post:v1", "message:v1")
    pub payload: DagPayload,  // Type-specific payload
    pub prev: Vec<String>,    // CIDs of parent nodes
    pub author: String,       // Author's public key (hex)
    pub timestamp: DateTime<Utc>,
    pub sig: String,          // ED25519 signature
}
```

**Supported Payload Types:**

| Type | Description |
|------|-------------|
| `Profile` | User profile with name, bio, founder ID, encryption pubkey |
| `Post` | Social media post with content, attachments, geohash |
| `Message` | Encrypted private message with ciphertext, nonce, ephemeral pubkey |
| `Token` | Token operations: Mint, Burn, TransferClaim, Escrow, MintReward |
| `Web` | SuperWeb page with URL, title, content, description, tags |
| `Name` | SNS name registration linking name to target |
| `Blob` | Binary data (images, files) with MIME type |
| `Listing` | Marketplace listing with title, description, price |
| `Contract` | Smart contract with code and init params |
| `ContractCall` | Contract method invocation |
| `Proof` | Verification proof for Proof-of-Humanity |

#### Store — `store.rs`

The storage layer provides:
- **SQLite** for native desktop (persistent)
- **In-Memory HashMap** for WASM/Web (session-only)

Key operations:
- `put_node()` / `get_node()` — Content-addressed storage
- `get_recent_posts()` / `get_local_posts()` — Feed queries
- `get_messages()` — Message retrieval between peers
- `get_balance()` — Token balance calculation
- `search_web_pages()` — Full-text search for SuperWeb content

#### Network — `network.rs`

Built on libp2p with custom behaviors:

| Protocol | Purpose |
|----------|---------|
| **mDNS** | Local peer discovery (desktop only) |
| **Gossipsub** | Content propagation across the network |
| **Request/Response** | Direct peer-to-peer block requests |
| **Kademlia DHT** | Distributed content discovery and storage replication |

#### Virtual Machine — `vm.rs` & `wasm.rs`

Smart contract execution environment:

1. **KV-Store Contracts** — Simple JSON state machine with `set` and `delete` operations
2. **WASM Contracts** — Full WebAssembly execution via wasmi
3. **Web Page Rendering** — WASM-based dynamic page generation

### Frontend Components

Built with Dioxus 0.7, a React-like framework for Rust:

| Component | Description |
|-----------|-------------|
| `App` | Root component with routing and state management |
| `HomeComponent` | Global/local feeds with post creation |
| `MessagingComponent` | Peer list and encrypted chat |
| `ProfileComponent` | User profile, wallet, contracts, settings |
| `BrowserComponent` | SuperWeb browser with address bar and search |
| `MarketplaceComponent` | Listings grid and create listing form |
| `GeohashComponent` | Location-based feed and discovery |
| `VerificationPage` | Founder claim and verification status |
| `NavComponent` | Navigation bar with verification gating |

### Command/Event Architecture

The backend uses an actor-like pattern with commands and events:

```
┌─────────────────┐     AppCmd      ┌─────────────────┐
│                 │ ───────────────▶│                 │
│    Frontend     │                 │     Backend     │
│   (Dioxus UI)   │ ◀───────────────│   (Event Loop)  │
│                 │    AppEvent     │                 │
└─────────────────┘                 └─────────────────┘
```

**Key Commands (AppCmd):**
- `Init` — Initialize backend and load stored data
- `PublishProfile` / `PublishPost` — Create content
- `SendMessage` / `FetchMessages` — Messaging
- `ClaimUBI` / `SendTokens` — Token operations
- `PublishWebPage` / `FetchWebPage` — SuperWeb
- `DeployContract` / `CallContract` — Smart contracts
- `Vouch` — Verify another user

**Key Events (AppEvent):**
- `PeerDiscovered` / `PeerConnected` — Network status
- `BlockReceived` — New content from network
- `MessageReceived` — New private message
- `BalanceUpdated` — Token balance changed
- `VerificationStatusChanged` — PoH status updated

---

## 🛠️ Tech Stack

| Category | Technology |
|----------|------------|
| **Language** | Rust 🦀 |
| **Frontend** | Dioxus 0.7 (React-like) |
| **Networking** | libp2p (TCP, DNS, WebSocket, Noise, Yamux) |
| **Discovery** | mDNS (local), Kademlia DHT (global) |
| **Pub/Sub** | Gossipsub |
| **Storage** | SQLite (native), In-Memory (WASM) |
| **Encryption** | X25519 key exchange, AES-256-GCM |
| **Signing** | ED25519 |
| **Smart Contracts** | wasmi (WASM runtime) |
| **Serialization** | serde, serde_json, CBOR |

---

## 📦 Getting Started

### Prerequisites

1. **Rust & Cargo** — Install via [rustup.rs](https://rustup.rs/)
2. **Dioxus CLI** — Install with:
   ```bash
   cargo install dioxus-cli
   ```

### Running Locally

1. **Clone the repository:**
   ```bash
   git clone <repo-url>
   cd superApp
   ```

2. **Run in development mode:**
   ```bash
   dx serve --desktop
   ```
   This starts the desktop application with hot-reloading.

3. **Build for release:**
   ```bash
   dx build --release --desktop
   ```

### Running Tests

```bash
cargo test
```

The test suite includes:
- DAG node creation and verification
- Profile publishing
- Vouching system
- Web page publishing and SNS resolution
- Block fetching and replication
- DHT web discovery
- Smart contract KV operations

---

## 🗺️ Roadmap

### ✅ Completed
- [x] Basic P2P Networking (mDNS, Gossipsub, Kademlia)
- [x] 5-Page UI (Home, Messaging, Profile, Browser, Marketplace)
- [x] Geohash Location Features
- [x] E2E Encrypted Messaging
- [x] Token Engine & UBI System
- [x] Proof-of-Humanity Verification
- [x] Storage Replication (10+ peers)
- [x] SuperWeb Core (`sp://` protocol)
- [x] Smart Contract VM (WASM + KV-Store)
- [x] Marketplace P2P Trading

### 🔄 In Progress
- [ ] Full SuperWeb Search Integration
- [ ] Advanced Marketplace Features
- [ ] WASM Dynamic Site Uploads

### 📋 Planned
- [ ] Mobile Support (iOS/Android via Dioxus Mobile)
- [ ] Web Client (WASM)
- [ ] File Sharing System
- [ ] Group Chats
- [ ] Reputation System
- [ ] Decentralized Moderation

---

## 📄 License

This project is licensed under a **Proprietary License**. See [LICENSE](LICENSE) for full terms.

**Key Points:**
- You may view and contribute to improve the code
- You may NOT fork, redistribute, or use code snippets in your own projects
- All contributions become the property of the project owner
- See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines

---

## 🤝 Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

---

<p align="center">
  Made with ❤️ and Rust
</p>
