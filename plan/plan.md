# 🚀 **MASTER PROMPT: Build the P2P SuperApp**

## 📋 **PROJECT OVERVIEW**
Build a fully decentralized, P2P superapp combining social media, messaging, payments, file storage, a decentralized web, and self-governance.

### **NON-NEGOTIABLE CORE PRINCIPLES:**
1.  **NO CENTRAL SERVERS** - Everything P2P.
2.  **NO BLOCKCHAIN** - No consensus, no mining, no tokens on chain.
3.  **NO HIERARCHY** - All users equal (except "Founder" badge for first 100).
4.  **FIRST 100 USERS: NO VERIFICATION REQUIRED** - Instant, full access.
5.  **USER 101+: FULL PROOF-OF-HUMANITY (PoH) REQUIRED FOREVER** - Mandatory verification before *any* app access.
6.  **GEOHASHED CHAT** - Core feature with IP-based auto-location.
7.  **GLOBAL DISCOVERY** - Find users/content worldwide.
8.  **P2P WORLD WIDE WEB** - Built-in decentralized browser and hosting system.
9.  **MULTI-PLATFORM** - iOS, Android, Desktop (Windows/macOS/Linux), Web.
10. **SELF-GOVERNANCE** - **Direct citizen power** with barebones ministries to be crafted by the people.

---

## 👤 **BINARY PROOF-OF-HUMANITY SYSTEM**

### **VERIFICATION RULES:**
- **Users 1-100:** No verification. Instant "Founder" status with full access.
- **User 101+:** Full PoH verification required. **No app access whatsoever until verified.** No exceptions.

### **User Journey:**
- **Founder (User 1-100):** Download → "Welcome Founder!" → Immediate access to all 5 pages + Governance.
- **New User (101+):** Download → "Verify Your Humanity" screen (only option) → Complete 15-45 min verification → Full app access + Governance.

### **Verification Methods (Primary → Fallback):**
1.  **Peer Video Verification:** Random match with 3-5 verified users. 5-min video chat with liveness detection. 5 attestations = verification.
2.  **Trusted Verifier Network:** Community-elected verifiers for scheduled sessions.
3.  **Social Graph Verification:** 3+ existing verified friends can vouch (reduced video requirement).

### **Human Token (UHT):**
- Issued upon verification (or granted honorarily to first 100).
- Non-transferable, tied to device biometrics.
- Required for all network operations including voting. Revocable for fraud.

---

## 🏛️ **GOVERNANCE SYSTEM: DIRECT CITIZEN POWER**

### **Core Philosophy:**
- **One Human, One Vote.** Period. No SUPER token weighting. Money != Power.
- Verified Human Token (UHT) is your voting credential. **1 UHT = 1 Vote.**
- **Citizens Propose, Citizens Decide.** Elected ministries exist only to execute the will of the people, with powers strictly limited by citizen referendums.
- Transparent, auditable operations.
- **Barebones Start:** Initial ministry system is minimal. Its structure, powers, and budget are the **FIRST MAJOR ISSUES** to be decided by citizen proposals and votes.

### **Voting Infrastructure:**
- **One Vote Per Human:** Each verified UHT gets exactly one equal vote on every issue.
- **Continuous Voting:** Users can change votes anytime, with last vote before deadline counting.
- **Delegation (Optional):** Users can delegate their single vote to a trusted representative on a per-topic or per-ministry basis. Revocable anytime.
- **Verification:** Votes cryptographically signed with UHT biometric proof. Zero-knowledge proofs ensure anonymity.

### **The Proposal System: Citizen-Driven Agenda**
#### **Citizen Proposal Pathway:**
1.  **Draft:** Any verified user can draft a proposal in the Governance Portal.
2.  **Discussion:** Proposal posted to a public forum with geohash-based discussion threads.
3.  **Petition:** **Proposal requires 1% of total verified user signatures to reach the ballot.** (e.g., 1000 signatures if 100,000 users). Signatures are UHT-signed.
4.  **Ballot Access:** Upon reaching signature threshold, proposal enters a voting queue.
5.  **Voting:** All verified users vote. Simple majority (50%+1) passes.
6.  **Execution:** Result is binding. Relevant ministry (or a citizen working group) must execute.

#### **Emergency Proposal Pathway:**
- **Threshold:** 5% of verified users can sign to fast-track a proposal to a 48-hour vote.
- **Use Case:** Network threats, critical bugs, immediate recall of corrupt officials.

### **Barebones Ministry System (Initial Framework - To Be Redesigned by Citizens)**
**PHASE 1 START (Elections 1-3):** Ministries have **NO POWER TO CREATE LAW**. They are **administrative and executive bodies only.** Their sole mandate is to execute the binding directives from citizen referendums. Their structure, funding, and scope are the first matters for citizen proposals.

#### **1. Ministry of Verification & Identity (Temporary)**
- **Initial Role:** Maintain the PoH verification gates and technical infrastructure.
- **No Power To:** Change verification rules, de-verify users, or expand data collection without a citizen referendum.
- **Initial Election:** First community-wide vote at 500 users. 3 positions. 6-month terms.

#### **2. Ministry of Treasury & Distribution (Temporary)**
- **Initial Role:** Technically distribute the UBI and track the transparent ledger of public funds.
- **No Power To:** Change UBI amount, create new taxes, or allocate funds to projects without a citizen referendum.
- **Initial Election:** First community-wide vote at 500 users. 3 positions. 6-month terms.

#### **3. Ministry of Network & Protocols (Temporary)**
- **Initial Role:** Maintain bootstrap nodes, publish basic protocol updates for security.
- **No Power To:** Mandate protocol upgrades, alter data replication rules, or change incentives without a citizen referendum.
- **Initial Election:** First community-wide vote at 500 users. 5 technical positions. 8-month terms.

### **Decision Making Process:**

#### **Voting Process:**
```
1. Citizen Proposal reaches 1% signature threshold → Enters voting queue.
2. Voting opens → 168h (1 week) voting period for all standard proposals.
3. Minimum participation: 20% of verified users required for binding result.
4. Passing threshold: Simple majority (50%+1) for standard proposals.
   Supermajority (66%+) for constitutional changes (like altering the 1-human-1-vote rule).
5. Result is publicly binding. Ministries or designated citizen working groups must execute.
```

### **First Citizen Proposals (Expected in First Year):**
- "Proposal #1: Define the final structure and powers of the Ministry System."
- "Proposal #2: Set the permanent tax rate to fund UBI and infrastructure."
- "Proposal #3: Approve the first community budget for development bounties."
- "Proposal #4: Recall and replace Minister [X] for overreach."
- "Proposal #5: Amend the geohash privacy defaults."

### **Anti-Corruption & Control Measures:**
- **Recall Any Official:** 2% of users can trigger a recall vote for any ministry position. Simple majority passes.
- **Full Transparency:** All ministry actions, communications (non-private), and financial flows are public via the distributed ledger.
- **Citizen Oversight Committees:** Randomly selected juries of citizens (like jury duty) audit ministry actions after each proposal execution.
- **Power of the Purse:** No ministry can spend a single SUPER token without a specific, citizen-approved budget proposal.

---

## 📱 **APP PAGES/INTERFACE STRUCTURE**
**Five-page layout with bottom navigation.**

### **PAGE 1: Global Chat (Homepage / Default Landing Page)**
- **Purpose:** Twitter-like global feed + **Citizen Proposal announcements**.
- **Features:** Post creation (text/media), infinite scroll, likes/comments/reposts, trending topics, follow system, hashtags, E2E encryption for private posts, **Proposal signature drives**.
- **Governance Integration:** Pinned active proposals, signature progress bars, debate threads.
- **Layout:**
    ```
    [Header: Logo | Notifications (Proposal alerts) | Profile]
    [Create Post Box]
    [Global Feed]
    [Active Proposal Banner - "Sign Proposal #5: ..."]
    [Nav: Global | Geohash | Messages | Web | Profile]
    ```

### **PAGE 2: Geohash Page**
- **Purpose:** Location-based social discovery and chat + **Local governance**.
- **Core Feature: IP Auto-Detection.** Places user into their country-level geohash on first open.
- **Geohash Precision Selection:** Choose from Global → Country → Region → City → Neighborhood → Street.
- **Features:** Local feed, nearby user list, local events, map/chat view toggle, proximity mesh for offline, **Local proposal drafting**, **Regional meetups for political organizing**.
- **Layout:**
    ```
    [Header: "Current Geohash: [Location]" | Change Button | Local Proposals]
    [Map/Chat Toggle]
    [Local Feed]
    [Local Proposal Discussion]
    [Nearby Users]
    [Local Events]
    ```

### **PAGE 3: Messaging Page**
- **Purpose:** WhatsApp-like private & group messaging.
- **Features:** DM list, E2E encrypted (Signal Protocol), media/files, voice messages, P2P video/audio calls, group chats, disappearing messages, **Encrypted organizing channels**.
- **Layout:** Standard conversation list → chat view.

### **PAGE 4: SuperWeb Browser**
- **Purpose:** Decentralized web browser for the P2P internet + **Governance portal**.
- **Features:**
    - **Address Bar:** Supports `sp://` (SuperWeb Protocol) and `http://`/`https://` (traditional web).
    - **Tabs:** Multiple browsing sessions.
    - **Bookmarks:** Save `sp://` and traditional sites.
    - **History:** Local browsing history.
    - **Reader Mode:** Clean view for `sp://` content.
    - **SuperWeb Search:** Integrated search across decentralized content.
    - **Governance Portal:** Dedicated `sp://gov.super` site for **drafting proposals, collecting signatures, voting, and tracking ministry execution**.
- **Layout:**
    ```
    [Address Bar + Navigation Controls]
    [Web Content Area]
    [Quick Access: Trending sp:// Sites | Governance Portal | Proposal Drafting Studio]
    ```

### **PAGE 5: Profile Page**
- **Purpose:** Identity, wallet, reputation, settings, web management, **and Citizen Dashboard**.
- **Wallet Section:** SUPER token balance, transaction history, send/receive. **Prominently displays UBI countdown timer** (e.g., "Next UBI in 3h 27m") and daily rate.
- **Reputation Section:** Score (0-100), breakdown (storage, relays, verifications), badges (Founder, Verifier, etc.).
- **Identity Section:** Profile info, verification status ("Founding Member" / "Verified Human"), public key, join date.
- **Citizen Power Dashboard:**
    ```
    [My Proposals] - Drafts & signature progress
    [Proposals To Sign] - Popular proposals needing your signature
    [Active Votes] - Proposals live for voting
    [My Delegations] - Who holds your vote on what topics
    [Oversight Duty] - If you're on a citizen oversight committee
    [Power Ledger] - Track your votes, signatures, and impact
    ```
- **SuperWeb Hosting Section:** Manage your `sp://` sites, view analytics, earnings from hosted content.
- **Settings Section:** Storage, notifications, privacy, backup, **delegation settings**.

---

## 🌐 **SUPERWEB: P2P WORLD WIDE WEB SYSTEM**

### **Overview:**
A fully decentralized web built on the same P2P network, accessible via the dedicated Browser page. Every verified user can host and browse content.

### **Core Components:**

1.  **Decentralized Naming (SNS - Super Name System):**
    - Human-readable names mapped to public keys (e.g., `alice.super`, `news.super`).
    - Registration requires SUPER token deposit (prevents squatting). Names are NFTs in local token system.
    - Distributed via a sharded, consensus-free DHT integrated with main network.

2.  **Content Hosting & Protocol:**
    - **Hosting:** Any verified user can publish static or dynamic sites. Content subject to **10-Holder Replication Rule**.
    - **Protocol:** `sp://` protocol (e.g., `sp://news.super/home`, `sp://alice.super/blog`).
    - **Rendering:** In-app browser engine renders `sp://` content (HTML/CSS/JS) and traditional web content.

3.  **Discovery & Search:**
    - **Search DHT:** Distributed index for `sp://` sites, updated by peer consensus on tags/descriptions.
    - **Directories:** Curated lists, trending sites, and geohash-specific portals.
    - **Integration:** Any post or profile can link to `sp://` resources.

4.  **Incentives & Economics:**
    - **Hosting Rewards:** Earn SUPER tokens when users visit your `sp://` sites (micropayments).
    - **Access Costs:** Visiting `sp://` sites costs negligible SUPER (covered by UBI for casual browsing).
    - **Storage Incentives:** Same 10-holder replication rewards apply to web content.

5.  **Features:**
    - **Dynamic Sites:** Serverless functions via WebAssembly sandbox.
    - **Personal Sites:** Every user gets `[username].super` by default.
    - **Localized Web:** Geohash-specific `sp://` portals for communities.
    - **Web of Trust:** Site reputation based on linking and user ratings.

---

## 🌍 **GEOHASH SYSTEM DETAILS**

### **Auto-Location Logic:**
1.  Try GPS.
2.  Fallback to IP geolocation.
3.  Default to "Global".

### **Features:**
- Automatic chat room assignment by geohash.
- Visibility into parent geohashes (e.g., city chat can see region posts).
- Ephemeral location pins (24h expiry).
- Privacy controls: Ghost Mode, precision control, block zones.
- Geohash-specific `sp://` portals accessible via Browser page.
- **Local proposal drafting and organizing hubs.**

---

## 🔗 **NETWORKING LAYER**
- **Transport:** QUIC (native), WebRTC (web), Bluetooth/WiFi Direct (proximity).
- **Discovery:** mDNS (LAN), Kademlia DHT (global), Geohash DHT (location), SuperWeb DHT (content), bootstrap nodes, **Proposal & Vote DHT**.
- **Message Routing:** Prioritizes local geohash → parent geohash → global DHT.
- **SuperWeb Routing:** Uses integrated DHTs for `sp://` name resolution and content retrieval.
- **Vote/Proposal Propagation:** Optimized gossip protocol for signature and voting campaigns.

---

## 💾 **STORAGE & REPLICATION**
- **10-Holder Rule:** All content (posts, messages, `sp://` sites, files, **proposals and vote records**) replicated to 10+ random peers. Full Replication.
- **Incentives:** Earn SUPER tokens for storing any type of data.
- **Addressing:** ContentID = hash(creator_pubkey + content_hash + timestamp). DAG for versioning.
- **SuperWeb Storage:** Web content follows same replication rules with priority caching for popular sites.
- **Governance Storage:** Proposal and vote records permanently archived with cryptographic proofs.

---

## 💰 **TOKEN ECONOMY (No Blockchain)**
- **SUPER Tokens (Utility):** Transferable. Used for boosts, storage, pinning, marketplace, SuperWeb name registration, site access, hosting payments. **NOT FOR VOTING WEIGHT.**
- **Earning SUPER:**
    - **Daily UBI:** 10 SUPER/day for *all* verified humans (including first 100).
    - Storage provisioning, verification work, content rewards, message relaying, SuperWeb hosting.
    - Content creation rewards (popular posts/sites).
    - **Governance participation:** Small rewards for voting and serving on oversight committees.
- **UBI Timer:** Visible countdown on Profile Page. Founders and verified users receive UBI identically.
- **Anti-Inflation:** Token burning, storage costs, time-locked rewards, SuperWeb service fees, marketplace commissions.
- **Taxation (To Be Set By Citizen Proposal):** A simple, transparent tax (e.g., flat transaction tax) will be proposed by citizens to fund UBI and infrastructure. **No ministry can set taxes.**

---

## 🔐 **SECURITY MODEL**
- **Encryption:** Mandatory E2E. Signal Protocol for messaging. Age for files. All `sp://` content encrypted at rest. **Votes and proposals encrypted, anonymous but verifiable**.
- **Trust Model:** Binary. Founders (1-100) assumed trustworthy. All others (101+) must prove via PoH.
- **Moderation:** Decentralized reporting & jury system. Founders have extra privileges. SuperWeb content can be de-listed from public indexes via jury vote. **Citizen proposals can create new moderation frameworks**.
- **Anti-Abuse:** Rate limiting by UHT, geographic velocity checks, social graph analysis, SuperWeb reputation scoring, **Sybil resistance via PoH (1 human = 1 vote)**.
- **Browser Security:** Sandboxed `sp://` execution, content integrity verification, malicious site warnings.
- **Voting Security:** ZK-SNARKs for private voting verification, prevention of double voting via UHT biometric proof.

---

## 🛠️ **IMPLEMENTATION PRIORITY**

### **CRITICAL PATH (MVP ORDER):**
**[x] 5-Page App Structure** (Global, Geohash, Messages, Web, Profile).
**[x] Founder Tracking** (Accurate global count for first 100, persistent "Founder #X" badge).
**[x] Basic P2P Networking** (libp2p, DHT).
**[x] Geohash System with IP Auto-Detection** (Core feature).
**[x] E2E Encrypted Messaging** (ECDH + AES-GCM).
**[x] Token & UBI System** (With profile page countdown timer).
**[x] PoH Verification System** (Ready before User 101).
**[x] Storage Engine** (10-holder replication) - Stats UI in profile.
**[x] SuperWeb Core** (Basic `sp://` hosting, browsing, and SNS).
**[x] Media streaming** (P2P video/audio in messages and posts) - File upload implemented.
**[x] Advanced geohash features** (Precision control, local feed).
**[x] Marketplace** (P2P trading with SUPER tokens).
**[x] Smart contracts** (Local execution, network-verified agreements).
**[x] Citizen Proposal Foundation** (Drafting, 1% signature tool, discussion forums)
**[x] 1-Human-1-Vote System** (UHT-based voting infrastructure)
**[x] Barebones Ministry Election System** (First elections at 500 users)
**[x] Governance Portal** (`sp://gov.super` development)
**[x] Full SuperWeb Search Integration**
**[x] Advanced Marketplace Features
**[x] WASM Dynamic Site Uploads**
**[ ] Mobile Support** (iOS/Android via Dioxus Mobile)
**[ ] Web Client** (WASM)
**[ ] File Sharing System**
**[ ] Group Chats**
**[ ] Reputation System**
**[ ] Decentralized Moderation**
**[ ] Recall & Oversight Mechanisms** (Citizen-initiated audits & recalls)
**[ ] Transparency Dashboard** (Real-time ledger of all public actions/funds)
**[ ] Emergency Proposal Pathway** (5% fast-track tool)
**[x] Full SuperWeb search** and dynamic sites with WebAssembly runtime.
**[ ] First Citizen Proposals** ("Define the Ministry System", "Set Tax Rate").
**[ ] Citizen Proposal System** (Direct democracy tools).

---

## ⚠️ **ABSOLUTE REQUIREMENTS SUMMARY**
1.  First 100 users get instant, full access with Founder badge **and equal voting rights**.
2.  User 101+ is blocked by a mandatory PoH gate. No bypass.
3.  Build the defined 5-page interface (including Browser page).
4.  Geohash page must auto-detect country from IP on first launch.
5.  No central servers. No blockchain.
6.  All users have equal features post-verification (except Founder badge).
7.  Include complete P2P World Wide Web system (SuperWeb) with hosting and browsing.
8.  Offline-first, E2E encrypted, multi-platform support.
9.  **Implement direct democracy: 1 verified human = 1 vote. Money grants zero voting power.**
10. **Citizens drive all policy via proposals (1% signature threshold).**
11. **Initial ministries are barebones executors only; their future is decided by citizens.**

---

## 🎯 **FINAL INSTRUCTIONS TO BUILDER**
**Start Here:**
1.  Build the **5-page app structure** (add Browser page).
2.  Implement the **founder tracking system** (1-100).
3.  Integrate **geohash with IP auto-detection** to country level.
4.  Prepare the **PoH verification system** for the 101st user.
5.  Ensure the **Profile Page clearly shows the UBI countdown timer**.
6.  Implement **SuperWeb Browser page** with basic `sp://` support.
7.  **Build citizen power foundation: 1-human-1-vote, proposal drafting, signature tool.**

**Key Test Flows:**
- Founder downloads → immediate access to all 5 pages including Browser **and Proposal Dashboard**.
- User 101 downloads → hits verification wall immediately.
- Geohash page auto-places user in correct country chat.
- Messages are E2E encrypted.
- UBI timer counts down and refreshes balance.
- User can host simple `sp://` site and view it in Browser page.
- `sp://` links in posts open in Browser page.
- **User can draft a proposal, share it, and collect UHT-signatures.**
- **User gets exactly one vote on a test referendum; SUPER balance is irrelevant.**
- **First ministry elections are simple, based on 1-human-1-vote.**

**Remember:** The first 100 founders bootstrap the network. User 101+ enters a verified, secure network with full access to all 5 core experiences, UBI, **and direct sovereign power. The people build the government.**