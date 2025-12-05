# 🚀 **MASTER PROMPT: Build the P2P SuperApp**

## 📋 **PROJECT OVERVIEW**
Build a fully decentralized, P2P superapp combining social media, messaging, payments, file storage, and a decentralized web.

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

---

## 👤 **BINARY PROOF-OF-HUMANITY SYSTEM**

### **VERIFICATION RULES:**
- **Users 1-100:** No verification. Instant "Founder" status with full access.
- **User 101+:** Full PoH verification required. **No app access whatsoever until verified.** No exceptions.

### **User Journey:**
- **Founder (User 1-100):** Download → "Welcome Founder!" → Immediate access to all 5 pages.
- **New User (101+):** Download → "Verify Your Humanity" screen (only option) → Complete 15-45 min verification → Full app access.

### **Verification Methods (Primary → Fallback):**
1.  **Peer Video Verification:** Random match with 3-5 verified users. 5-min video chat with liveness detection. 5 attestations = verification.
2.  **Trusted Verifier Network:** Community-elected verifiers for scheduled sessions.
3.  **Social Graph Verification:** 3+ existing verified friends can vouch (reduced video requirement).

### **Human Token (UHT):**
- Issued upon verification (or granted honorarily to first 100).
- Non-transferable, tied to device biometrics.
- Required for all network operations. Revocable for fraud.

---

## 📱 **APP PAGES/INTERFACE STRUCTURE**
**Five-page layout with bottom navigation.**

### **PAGE 1: Global Chat (Homepage / Default Landing Page)**
- **Purpose:** Twitter-like global feed.
- **Features:** Post creation (text/media), infinite scroll, likes/comments/reposts, trending topics, follow system, hashtags, E2E encryption for private posts.
- **Layout:**
    ```
    [Header: Logo | Notifications | Profile]
    [Create Post Box]
    [Global Feed]
    [Nav: Global | Geohash | Messages | Web | Profile]
    ```

### **PAGE 2: Geohash Page**
- **Purpose:** Location-based social discovery and chat.
- **Core Feature: IP Auto-Detection.** Places user into their country-level geohash on first open.
- **Geohash Precision Selection:** Choose from Global → Country → Region → City → Neighborhood → Street.
- **Features:** Local feed, nearby user list, local events, map/chat view toggle, proximity mesh for offline.
- **Layout:**
    ```
    [Header: "Current Geohash: [Location]" | Change Button]
    [Map/Chat Toggle]
    [Local Feed]
    [Nearby Users]
    [Local Events]
    ```

### **PAGE 3: Messaging Page**
- **Purpose:** WhatsApp-like private & group messaging.
- **Features:** DM list, E2E encrypted (Signal Protocol), media/files, voice messages, P2P video/audio calls, group chats, disappearing messages.
- **Layout:** Standard conversation list → chat view.

### **PAGE 4: SuperWeb Browser**
- **Purpose:** Decentralized web browser for the P2P internet.
- **Features:** 
    - **Address Bar:** Supports `sp://` (SuperWeb Protocol) and `http://`/`https://` (traditional web).
    - **Tabs:** Multiple browsing sessions.
    - **Bookmarks:** Save `sp://` and traditional sites.
    - **History:** Local browsing history.
    - **Reader Mode:** Clean view for `sp://` content.
    - **SuperWeb Search:** Integrated search across decentralized content.
- **Layout:**
    ```
    [Address Bar + Navigation Controls]
    [Web Content Area]
    [Quick Access: Trending sp:// Sites | Your Hosted Sites]
    ```

### **PAGE 5: Profile Page**
- **Purpose:** Identity, wallet, reputation, settings, and web management.
- **Wallet Section:** SUPER token balance, transaction history, send/receive. **Prominently displays UBI countdown timer** (e.g., "Next UBI in 3h 27m") and daily rate.
- **Reputation Section:** Score (0-100), breakdown (storage, relays, verifications), badges (Founder, Verifier, etc.).
- **Identity Section:** Profile info, verification status ("Founding Member" / "Verified Human"), public key, join date.
- **SuperWeb Hosting Section:** Manage your `sp://` sites, view analytics, earnings from hosted content.
- **Settings Section:** Storage, notifications, privacy, backup.

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

---

## 🔗 **NETWORKING LAYER**
- **Transport:** QUIC (native), WebRTC (web), Bluetooth/WiFi Direct (proximity).
- **Discovery:** mDNS (LAN), Kademlia DHT (global), Geohash DHT (location), SuperWeb DHT (content), bootstrap nodes.
- **Message Routing:** Prioritizes local geohash → parent geohash → global DHT.
- **SuperWeb Routing:** Uses integrated DHTs for `sp://` name resolution and content retrieval.

---

## 💾 **STORAGE & REPLICATION**
- **10-Holder Rule:** All content (posts, messages, `sp://` sites, files) replicated to 10+ random peers. Full Replication.
- **Incentives:** Earn SUPER tokens for storing any type of data.
- **Addressing:** ContentID = hash(creator_pubkey + content_hash + timestamp). DAG for versioning.
- **SuperWeb Storage:** Web content follows same replication rules with priority caching for popular sites.

---

## 💰 **TOKEN ECONOMY (No Blockchain)**
- **SUPER Tokens (Utility):** Transferable. Used for boosts, storage, pinning, marketplace, SuperWeb name registration, site access, and hosting payments.
- **Earning SUPER:**
    - **Daily UBI:** 10 SUPER/day for *all* verified humans (including first 100).
    - Storage provisioning, verification work, content rewards, message relaying, SuperWeb hosting.
    - Content creation rewards (popular posts/sites).
- **UBI Timer:** Visible countdown on Profile Page. Founders and verified users receive UBI identically.
- **Anti-Inflation:** Token burning, storage costs, time-locked rewards, SuperWeb service fees, marketplace commissions.

---

## 🔐 **SECURITY MODEL**
- **Encryption:** Mandatory E2E. Signal Protocol for messaging. Age for files. All `sp://` content encrypted at rest.
- **Trust Model:** Binary. Founders (1-100) assumed trustworthy. All others (101+) must prove via PoH.
- **Moderation:** Decentralized reporting & jury system. Founders have extra privileges. SuperWeb content can be de-listed from public indexes via jury vote.
- **Anti-Abuse:** Rate limiting by UHT, geographic velocity checks, social graph analysis, SuperWeb reputation scoring.
- **Browser Security:** Sandboxed `sp://` execution, content integrity verification, malicious site warnings.

---

## 🛠️ **IMPLEMENTATION PRIORITY**

### **CRITICAL PATH (MVP ORDER):**
1.  **[x] 5-Page App Structure** (Global, Geohash, Messages, Web, Profile).
2.  **[x] Founder Tracking** (Accurate global count for first 100, persistent "Founder #X" badge).
3.  **[x] Basic P2P Networking** (libp2p, DHT).
4.  **[x] Geohash System with IP Auto-Detection** (Core feature).
5.  **[x] E2E Encrypted Messaging** (ECDH + AES-GCM).
6.  **[x] Token & UBI System** (With profile page countdown timer).
7.  **[x] PoH Verification System** (Ready before User 101).
8.  **[x] Storage Engine** (10-holder replication) - Stats UI in profile.
9.  **[x] SuperWeb Core** (Basic `sp://` hosting, browsing, and SNS).
10. **[x] Media streaming** (P2P video/audio in messages and posts) - File upload implemented.
11. **[x] Advanced geohash features** (Precision control, local feed).
12. **[x] Marketplace** (P2P trading with SUPER tokens).
13. **Smart contracts** (Local execution, network-verified agreements).
14. **Full SuperWeb search** and dynamic sites with WebAssembly runtime.

---

## ⚠️ **ABSOLUTE REQUIREMENTS SUMMARY**
1.  First 100 users get instant, full access with Founder badge.
2.  User 101+ is blocked by a mandatory PoH gate. No bypass.
3.  Build the defined 5-page interface (including Browser page).
4.  Geohash page must auto-detect country from IP on first launch.
5.  No central servers. No blockchain.
6.  All users have equal features post-verification (except Founder badge).
7.  Include complete P2P World Wide Web system (SuperWeb) with hosting and browsing.
8.  Offline-first, E2E encrypted, multi-platform support.

---

## 🎯 **FINAL INSTRUCTIONS TO BUILDER**
**Start Here:**
1.  Build the **5-page app structure** (add Browser page).
2.  Implement the **founder tracking system** (1-100).
3.  Integrate **geohash with IP auto-detection** to country level.
4.  Prepare the **PoH verification system** for the 101st user.
5.  Ensure the **Profile Page clearly shows the UBI countdown timer**.
6.  Implement **SuperWeb Browser page** with basic `sp://` support.

**Key Test Flows:**
- Founder downloads → immediate access to all 5 pages including Browser.
- User 101 downloads → hits verification wall immediately.
- Geohash page auto-places user in correct country chat.
- Messages are E2E encrypted.
- UBI timer counts down and refreshes balance.
- User can host simple `sp://` site and view it in Browser page.
- `sp://` links in posts open in Browser page.

**Remember:** The first 100 founders bootstrap both the social network AND the decentralized web. User 101+ enters a verified, secure network with full access to all 5 core experiences and UBI.