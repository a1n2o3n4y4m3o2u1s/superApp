## 🧠 NEW CORE PRINCIPLE

> **SuperWeb is the app. Everything else is a module rendered inside it.**

No redirects. No context switching. No “different apps.”

---

## 🌐 SUPERWEB-FIRST ARCHITECTURE (UPDATED)

### Single Entry Point

```
sp://super.app
```

This is the **only “homepage.”**

All functionality loads as **internal SuperWeb modules**:

* Social
* Local
* Governance
* Market
* Education
* Messages
* Profile

Rendered via WASM components inside the SuperWeb shell.

---

## 🧩 MODULED APP STRUCTURE (CONSOLIDATED)

### 1. 🏠 Home Module — Global Social Layer (PRIMARY FEED)

**This replaces the old “Home” page.**

**Behavior:**

* Shows **all posts from all geohashes**
* Chronological by default
* Optional filters:

  * Global
  * My geohash
  * Followed users
  * Topics / tags

**Posting logic (critical change):**

* Posts created here are **Global Posts**
* Global Posts:

  * Appear in Home feed
  * Appear in **every relevant geohash feed**
  * Are **byte-identical and synchronized**
  * No duplication, no forks

Geohash feeds are now **filtered views of the same global dataset**, not separate systems.

---

### 2. 📍 Local Module — Geohash Views (FILTER, NOT SEPARATE SYSTEM)

**Local is no longer a separate universe.**

It is a **contextual filter** on global data:

* Local social feed (subset of global posts)
* Local stories
* Local chats
* Local proposals
* Local marketplace listings
* Local SuperWeb portals (`sp://[geohash].super`)

Offline mesh chat remains intact.

---

### 3. 🌐 SuperWeb Module — Web, Apps, and Hosting (CORE)

SuperWeb now **contains everything**:

* Social feeds
* Governance dashboards
* Market listings
* Education portals
* Messages
* User profiles

**Capabilities:**

* `sp://` browsing
* `.super` domains
* User-hosted WASM apps
* Internal routing (no redirects)
* Embedded governance + voting UIs
* Embedded education & certification UIs

---

### 4. 🏛 Govern Module — Direct Democracy (Embedded)

Rendered inside SuperWeb:

* Proposal drafting
* Signature collection
* Voting
* Delegation
* Ministry dashboards
* Recall mechanisms
* Transparency reports

Education certifications (see below) can gate **participation types**, not voting rights.

---

### 5. 💰 Market Module — Economy + Services

Integrated tightly with Education:

* Goods & services
* Digital products
* Hosting
* Storage leasing
* **Certified services** (see Education)
* SUPER-only payments

---

### 6. 💬 Messages Module

Unchanged technically, but now:

* Accessed via SuperWeb overlay
* Context-aware (local, governance, education groups)

---

### 7. 👤 Profile Module — Identity Hub

Single unified dashboard:

* Wallet + UBI
* Reputation
* Badges & certifications
* Governance participation
* Hosting manager
* Privacy controls

---

## 🎓 NEW: EDUCATION & CERTIFICATION SYSTEM (FULL DESIGN)

### Purpose

To enable **skill verification, civic competence, and service trust** without hierarchy or central authority.

### Structure

#### 📚 Education Portals (SuperWeb-native)

* Hosted as `sp://edu.*.super`
* Anyone can create a curriculum
* Content types:

  * Courses
  * Reading lists
  * Simulations
  * Civic training
  * Technical training

#### 🧪 Certification Mechanism

* Exams are:

  * Peer-reviewed
  * Open-source
  * Deterministic
* Certification issuance:

  * Requires quorum of certified peers
  * Cryptographically signed
  * Stored in profile (non-transferable)

#### 🏷 Certification Types

* Civic literacy
* Governance roles
* Technical skills
* Trade qualifications
* Moderation / jury eligibility

---

### 🔗 Integration Points

**Profile**

* [x] Certifications displayed as badges

**Market**

* [x] Services can require certifications
* [x] Buyers can filter by certified providers

**Governance**

* Certifications can:

  * [x] Gate proposal authorship types (Constitutional amendments etc.)
  * [x] Gate ministry execution roles (Candidacy)
* **Never gate voting**

**Reputation**

* Certifications enhance trust, not power

---

## 🧠 SOCIAL SYSTEM (UPDATED LOGIC)

* One unified post object
* Tagged with:

  * Author
  * Timestamp
  * Geohash scope(s)
  * Topic tags

Feeds are **queries**, not silos.

Replication still follows the 10-holder rule.

---

## 📐 PAGE COUNT REDUCTION (RESULT)

**Before:**
7 hard pages + subpages

**After:**
1 SuperWeb shell + 7 logical modules rendered internally

This dramatically simplifies UX without reducing capability.

---

## 🧱 CORE RULES — STILL ENFORCED

Unchanged and intact:

* No servers
* No blockchain
* No hierarchy
* PoH gate for users 101+
* 1 human = 1 vote
* Equal rights after verification
* E2E encryption everywhere
* Citizen-driven governance
* Ministries execute only

---

## ✅ IMPLEMENTATION PROGRESS

### Completed Modules

| Module | Status | Features |
|--------|--------|----------|
| 🏠 Home | ✅ Done | Social feed, posts, likes, comments, stories, follow/unfollow |
| 📍 Local | ✅ Done | Geohash filtering, local feed |
| 🌐 SuperWeb | ✅ Done | Browser, sp:// protocol, WASM hosting |
| 🏛 Govern | ✅ Done | Proposals, voting, elections, recalls, oversight, ministries |
| 💰 Market | ✅ Done | Listings, marketplace |
| 💬 Messages | ✅ Done | E2E encrypted messaging, groups |
| 👤 Profile | ✅ Done | Identity, wallet, UBI, files, reputation |
| 🎓 Education | ✅ Done | Courses, exams, exam-taking, certifications |

### Latest Updates

- **2025-12-13**: Refactored to SuperWeb-first navigation - removed top tabs, created wiki homepage with pinned apps and community pages
- **2025-12-13**: Completed exam-taking functionality with `ExamTakingModal`, score calculation, and result display
- **2025-12-13**: Refined Education UI with `CourseDetailComponent` and `CreateExamForm` for better course authoring and viewing experience
- **2025-12-13**: Implemented Certification Gating for Governance (Proposals & Candidacy) requiring specific certifications.
- **2025-12-20**: Integrated Smart Contracts into SuperWeb-first architecture; added to homepage pinned apps and updated Marketplace navigation.