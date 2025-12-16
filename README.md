# 🌌 P2P SuperApp: The Post-Cloud OS

<p align="center">
  <strong>Complete Digital Sovereignty. Zero Servers. 100% Peer-to-Peer.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.75+-black?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Stack-Libp2p%20%7C%20Dioxus%20%7C%20Tokio-blue?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Architecture-DAG%20%2B%20Gossipsub-purple?style=for-the-badge" />
  <img src="https://img.shields.io/badge/Security-AES256%20%7C%20X25519-green?style=for-the-badge" />
</p>

---

## 📜 Overview

**P2P SuperApp** is a monolithic decentralized application that replaces the entire cloud stack (Social, Commerce, Governance, Messaging) with a single local-first binary. It runs no central servers; every feature relies on a custom **Directed Acyclic Graph (DAG)** synced via **Libp2p Gossipsub**.

This is not just a social app; it is a **Digital Nation State** in a box, complete with its own constitution, economy, education system, and sovereign identity layer.

---

## 🏗️ System Architecture

The application is split into a **Reactive Frontend** (Dioxus) and an **Async Backend** (Tokio/Libp2p), communicating via message passing (`AppCmd` <-> `AppEvent`).

### 1. The Data Layer (DAG)
All data is stored as immutable **DAG Nodes** (Content-Addressable).
- **Structure**: `id` (CID), `prev` (Causality), `author` (PubKey), `sig` (Ed25519), `payload`.
- **Storage**: SQLite (`blocks` table for data, `blocks_meta` for fast indexing, `blobs` for large media).
- **Sync**: 
    - **Gossipsub**: Real-time multicasting of new blocks on topic `blocks`.
    - **Request-Response**: Direct fetching of missing history or blob content.

### 2. The Network Layer
- **Transport**: TCP + WebSocket + DNS (for global reach).
- **Discovery**: 
    - **Global**: Kademlia DHT (bootstrapped via IPFS public nodes).
    - **Local**: mDNS (LAN) + Active Presence Heartbeats (`geohash:<prefix>` topics).
- **Encryption**: Noise Protocol (Authentication) + Yamux (Stream Multiplexing).

### 3. The Application Layer (Dioxus)
A single-page application (SPA) rendered natively on Desktop (via WebView).
- **State**: Global `AppState` signals tracked by Dioxus.
- **Routing**: Custom `SuperWebShell` router handling `sp://` URIs.

---

## 📱 The 10-Module Experience

The application is accessible via the **SuperWeb Browser** (`sp://`), which serves as the OS shell.

### 1. 🏠 Home (`sp://home.super`)
*Source: `src/components/home_page.rs`*
A decentralized social feed.
- **Protocol**: `post:v1`, `comment:v1`, `like:v1`, `story:v1`, `follow:v1`.
- **Features**:
    - **Ephemeral Stories**: 24h media posts. The backend auto-prunes expired stories.
    - **Blob Offloading**: Large images are stored in the `blobs` table, keeping the DAG metadata light.
    - **Follow Graph**: Subscription model based on causality.
    - **Official Announcements**: Elected officials can broadcast high-visibility updates.

### 2. 📍 Local (`sp://local.super`)
*Source: `src/components/geohash_page.rs`*
Hyper-local networking based on Geohashes.
- **Protocol**: `post:v1` (with `geohash` field), `AnnouncePresence`.
- **Discovery Logic**: 
    1. **Auto-Detect**: Backend resolves IP to Geohash.
    2. **Subscribe**: Listens to `geohash:<prefix>` topic.
    3. **Heartbeat**: Broadcasts `PRESENCE` every 60s to announce existence to neighbors.
- **Features**: Precision Zoom (Country -> Neighborhood), Local Feed, "Users Nearby" list.

### 3. 🌐 Browser (`sp://browser.super`)
*Source: `src/components/browser_page.rs` & `vm.rs`*
A decentralized web engine.
- **Protocol**: `web:v1`.
- **Engine**: 
    - **Content**: Parses Markdown/HTML stored in DAG nodes.
    - **WASM**: Executes sandboxed `\0asm` binaries for dynamic apps (`vm.rs`).
    - **Search**: Distributed index over `tags`.
- **Wiki Directory**: `sp://welcome` lists all published pages.

### 4. 🏪 Market (`sp://market.super`)
*Source: `src/components/marketplace_page.rs`*
Trust-minimized Global Marketplace.
- **Protocol**: `listing:v1`, `token:v1` (Transfers).
- **Features**: 
    - **Atomic Orders**: "Buy Now" triggers immediate settlement.
    - **Certified Filters**: Filter listings by sellers with specific Education Certifications.
    - **Contracts Integration**: Direct link to Smart Contracts for advanced trade logic.

### 5. 🏛️ Govern (`sp://gov.super`)
*Source: `src/components/governance_page.rs`*
The Operating System's Constitution.
- **Protocol**: `proposal:v1`, `vote:v1`, `candidacy:v1`, `recall:v1`, `oversight_case:v1`.
- **Mechanisms**:
    - **Taxation**: Citizens vote on `SetTax` proposals to adjust network fees.
    - **Ministries**: `DefineMinistries` proposals structure the executive branch.
    - **Elections**: Continuous voting for implementation candidates; requires `GovernanceRoles` certification.
    - **Recalls**: Immediate removal of bad actors.
    - **Jury Duty**: Random selection for content disputes (Ledger visible).
    - **Official Powers**: Active officials have unique capabilities:
        - **Pin Proposals**: Elevate priority legislation to the top of the queue.
        - **Broadcast**: Post official announcements to the global feed.

### 6. 💬 Messages (`sp://messages.super`)
*Source: `src/components/messaging_page.rs`*
End-to-End Encrypted Messaging (Signal-style).
- **Protocol**: `message:v1`, `group:v1`, `file:v1`.
- **Architecture**:
    - **Keys**: X25519 Ephemeral Key Exchange.
    - **Cipher**: AES-256-GCM.
    - **File Sharing**: Files are chunked, encrypted as blobs, and shared via `[FILE:cid:key:nonce:mime]` links.
    - **Rich UX**: 
        - **Unified Inbox**: Single searchable list for both Direct Messages and Groups.
        - **Quick Actions**: Dedicated "New Chat" and "New Group" flows for rapid connection.
        - **Media**: Secure image sharing with encrypted blob storage.
        - **Premium UI**: Desktop-class interface with glassmorphism, slide-in animations, and read receipts ("✓✓").

### 7. 👤 Profile (`sp://profile.super`)
*Source: `src/components/profile_page.rs`*
- **Protocol**: `profile:v1`, `proof:v1`.
- **Features**:
    - **Reputation**: Computed score based on Verification + Contribution.
    - **UBI**: Daily "Claim" logic for verified humans.
    - **Developer Console**: Deploy/Call WASM Smart Contracts (`contract:v1`, `contract_call:v1`).

### 8. 🎓 Education (`sp://edu.super`)
*Source: `src/components/education_page.rs`*
A complete Learning Management System (LMS) on-chain.
- **Protocol**: `course:v1`, `exam:v1`, `certification:v1`.
- **Flow**:
    1. **Create Course**: Markdown content, custom categories (e.g., CivicLiteracy).
    2. **Create Exam**: Multiple choice questions, passing score threshold.
    3. **Certify**: Passing an exam mints a `certification:v1` DAG node, unlocking Governance roles.

### 9. ✅ Verification (`sp://verify.super`)
*Source: `src/components/verification_page.rs`*
The gateway to citizenship.
- **Status**: `Unverified` -> `EligibleForFounder` -> `Founder` (First 100) -> `Verified`.
- **Process**: Submit Application (Bio/Photo) -> Community Vouching.
- **Grants**: UBI Eligibility, Voting Rights.

### 10. 📊 Transparency (`sp://transparency.super`)
*Source: `src/components/transparency_page.rs`*
Real-time "Block Explorer" for the nation.
- **Public Ledger**: Live stream of all `Token`, `Vote`, `Proposal`, and `Contract` events.
- **Network Stats**: Total Blocks & Storage used.

---

## 🛠️ Developer Guide

### Prerequisites
- **Rust**: `rustup update stable`
- **Dioxus CLI**: `cargo install dioxus-cli`
- **Dependencies**: `openssl`, `pkg-config`, `sqlite3`

### Build Instructions

#### MacOS / Linux
```bash
# Run Dev Server (Fast Rebuilds)
dx serve --desktop

# Release Build
dx build --release --desktop
```

#### Windows
Uses `build-windows.yml` workflow.
1. Requires `assets/` directory with `main.css`.
2. Requires `icons/icon.ico`.
3. Build artifact: `.msi` installer.

### Codebase Map
```
src/
├── main.rs                 # Frontend Entrypoint (SuperWebShell Router)
├── backend/
│   ├── mod.rs              # Event Loop, Command Handlers, P2P Swarm
│   ├── network.rs          # Libp2p Configuration (TCP, Noise, Yamux, Kademlia)
│   ├── store.rs            # SQLite Database & Blob Storage logic
│   ├── dag.rs              # DAG Node Structs & Payload Enums (The "Protocol")
│   ├── identity.rs         # Keypair Management (Ed25519)
│   ├── vm.rs               # WASM Runtime for Smart Contracts
│   └── wasm.rs             # WebAssembly Logic (if compiling for web target)
└── components/             # UI Components
    ├── superweb_shell.rs   # Main Router (sp://)
    ├── education_page.rs   # LMS System
    ├── transparency_page.rs# Public Ledger
    ├── browser_page.rs     # Web Engine
    ├── ...
```

---

## 🔍 Protocol Reference (Payload Types)

| Payload | Description |
| :--- | :--- |
| `profile:v1` | User Identity (Name, Bio, Avatar) |
| `post:v1` | Social Content (Text + Media CIDs) |
| `story:v1` | Ephemeral 24h Content |
| `message:v1` | Encrypted DM |
| `group:v1` | Group Chat Meta |
| `token:v1` | Currency Transfer |
| `web:v1` | Decentralized Website |
| `listing:v1` | Marketplace Item |
| `proposal:v1` | Governance Proposal (Law) |
| `vote:v1` | Vote on Proposal |
| `candidacy:v1` | Election Candidacy |
| `recall:v1` | Vote to Remove Official |
| `course:v1` | Educational Content |
| `exam:v1` | Exam Config |
| `exam_submission:v1` | Student Answers |
| `certification:v1` | Proof of Skill |
| `contract:v1` | WASM Smart Contract Code |

---

## 🔐 Security & Privacy

- **Encryption**: All private data (Messages, Files) is encrypted client-side using **AES-256-GCM**.
- **Anonymity**: Network traffic is encrypted (Noise). No central metadata server.
- **Verification**: Proof-of-Humanity system separates Humans from Bots for UBI/Voting.
- **Sovereignty**: Users can export/delete their local `store.db`.

---

**Generated by Antigravity Agent** | *System v0.1.0*
