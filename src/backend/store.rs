#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, Result};
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};
use std::path::Path;
use crate::backend::dag::{DagNode, DagPayload};
use serde_json;

#[derive(Clone)]
pub struct Store {
    #[cfg(not(target_arch = "wasm32"))]
    conn: std::sync::Arc<std::sync::Mutex<Connection>>,
    #[cfg(target_arch = "wasm32")]
    blocks: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    #[cfg(target_arch = "wasm32")]
    heads: Arc<Mutex<HashMap<String, String>>>,
}

impl Store {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id TEXT PRIMARY KEY,
                data BLOB
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS heads (
                public_key TEXT PRIMARY KEY,
                cid TEXT
            )",
            [],
        )?;

        Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new<P: AsRef<Path>>(_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            blocks: Arc::new(Mutex::new(HashMap::new())),
            heads: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open_in_memory()?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id TEXT PRIMARY KEY,
                data BLOB
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS heads (
                public_key TEXT PRIMARY KEY,
                cid TEXT
            )",
            [],
        )?;

        Ok(Self { conn: std::sync::Arc::new(std::sync::Mutex::new(conn)) })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_in_memory() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            blocks: Arc::new(Mutex::new(HashMap::new())),
            heads: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn put_node(&self, node: &DagNode) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(node)?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO blocks (id, data) VALUES (?1, ?2)",
                params![node.id, data],
            )?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut blocks = self.blocks.lock().unwrap();
            blocks.insert(node.id.clone(), data);
        }

        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Result<Option<DagNode>, Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT data FROM blocks WHERE id = ?1")?;
            
            let mut rows = stmt.query(params![id])?;

            if let Some(row) = rows.next()? {
                let data: Vec<u8> = row.get(0)?;
                let node: DagNode = serde_json::from_slice(&data)?;
                Ok(Some(node))
            } else {
                Ok(None)
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let blocks = self.blocks.lock().unwrap();
            if let Some(data) = blocks.get(id) {
                let node: DagNode = serde_json::from_slice(data)?;
                Ok(Some(node))
            } else {
                Ok(None)
            }
        }
    }

    pub fn get_block_bytes(&self, id: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
             let conn = self.conn.lock().unwrap();
             let mut stmt = conn.prepare("SELECT data FROM blocks WHERE id = ?1")?;
             let mut rows = stmt.query(params![id])?;
             if let Some(row) = rows.next()? {
                 let data: Vec<u8> = row.get(0)?;
                 Ok(Some(data))
             } else {
                 Ok(None)
             }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let blocks = self.blocks.lock().unwrap();
            Ok(blocks.get(id).cloned())
        }
    }


    pub fn update_head(&self, public_key: &str, cid: &str) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO heads (public_key, cid) VALUES (?1, ?2)",
                params![public_key, cid],
            )?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut heads = self.heads.lock().unwrap();
            heads.insert(public_key.to_string(), cid.to_string());
        }
        Ok(())
    }

    pub fn get_head(&self, public_key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT cid FROM heads WHERE public_key = ?1")?;
            
            let mut rows = stmt.query(params![public_key])?;

            if let Some(row) = rows.next()? {
                let cid: String = row.get(0)?;
                Ok(Some(cid))
            } else {
                Ok(None)
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let heads = self.heads.lock().unwrap();
            Ok(heads.get(public_key).cloned())
        }
    }

    pub fn get_all_nodes(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT data FROM blocks")?;
            let node_iter = stmt.query_map([], |row| {
                let data: Vec<u8> = row.get(0)?;
                Ok(data)
            })?;
            
            let mut nodes = Vec::new();
            for node_result in node_iter {
                let data = node_result?;
                if let Ok(node) = serde_json::from_slice::<DagNode>(&data) {
                    nodes.push(node);
                }
            }
            Ok(nodes)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let blocks = self.blocks.lock().unwrap();
            let mut nodes = Vec::new();
            for data in blocks.values() {
                 if let Ok(node) = serde_json::from_slice::<DagNode>(data) {
                    nodes.push(node);
                }
            }
            Ok(nodes)
        }
    }

    // Helper to replace repetitive queries
    pub fn get_recent_posts(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut posts: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "post:v1")
            .collect();

        posts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if posts.len() > limit {
            posts.truncate(limit);
        }
        Ok(posts)
    }

    /// Get posts filtered by geohash prefix
    pub fn get_local_posts(&self, geohash_prefix: &str, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut posts: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if n.r#type == "post:v1" {
                    if let DagPayload::Post(ref post) = n.payload {
                        if let Some(ref gh) = post.geohash {
                            return gh.starts_with(geohash_prefix);
                        }
                    }
                }
                false
            })
            .collect();

        posts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if posts.len() > limit {
            posts.truncate(limit);
        }
        Ok(posts)
    }

    /// Get active marketplace listings, ensuring we only show the latest version of each listing
    pub fn get_active_listings(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut listings: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "listing:v1")
            .collect();
            
        // Sort by timestamp descending so latest is first
        listings.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let mut latest_listings_map: std::collections::HashMap<String, DagNode> = std::collections::HashMap::new();

        for node in listings {
            if let DagPayload::Listing(ref listing) = node.payload {
                // The "Chain ID" is the ref_cid if it exists (meaning it points to the original),
                // OR the node's own ID if it has no ref_cid (meaning it IS the original).
                let chain_id = listing.ref_cid.clone().unwrap_or(node.id.clone());
                
                if !latest_listings_map.contains_key(&chain_id) {
                    latest_listings_map.insert(chain_id, node.clone());
                }
            }
        }
        
        // Filter for only Active status
        let mut active_listings: Vec<DagNode> = latest_listings_map.into_values()
            .filter(|node| {
                if let DagPayload::Listing(ref listing) = node.payload {
                    listing.status == crate::backend::dag::ListingStatus::Active
                } else {
                    false
                }
            })
            .collect();

        active_listings.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if active_listings.len() > limit {
            active_listings.truncate(limit);
        }
        Ok(active_listings)
    }

    pub fn search_listings(&self, query: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut listings: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "listing:v1")
            .collect();
            
        // Sort by timestamp descending so latest is first
        listings.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        let mut latest_listings_map: std::collections::HashMap<String, DagNode> = std::collections::HashMap::new();

        for node in listings {
            if let DagPayload::Listing(ref listing) = node.payload {
                let chain_id = listing.ref_cid.clone().unwrap_or(node.id.clone());
                if !latest_listings_map.contains_key(&chain_id) {
                    latest_listings_map.insert(chain_id, node.clone());
                }
            }
        }
        
        // Filter by query and Active status
        let query_lower = query.to_lowercase();
        let mut results: Vec<DagNode> = latest_listings_map.into_values()
            .filter(|node| {
                if let DagPayload::Listing(ref listing) = node.payload {
                    if listing.status == crate::backend::dag::ListingStatus::Active {
                         listing.title.to_lowercase().contains(&query_lower) || listing.description.to_lowercase().contains(&query_lower)
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .collect();

        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        Ok(results)
    }

    pub fn get_messages(&self, my_id: &str, other_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut messages: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if let DagPayload::Message(ref msg) = n.payload {
                    let is_from_me = n.author == my_id;
                    let is_to_me = msg.recipient == my_id;
                    let is_from_other = n.author == other_id;
                    let is_to_other = msg.recipient == other_id;
                    (is_from_me && is_to_other) || (is_from_other && is_to_me)
                } else {
                    false
                }
            })
            .collect();

        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(messages)
    }

    pub fn get_pending_transfers(&self, my_pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut incoming_burns: std::collections::HashMap<String, DagNode> = std::collections::HashMap::new();
        let mut my_claims: std::collections::HashSet<String> = std::collections::HashSet::new();

        for node in nodes {
             if let DagPayload::Token(ref token) = node.payload {
                match token.action {
                    crate::backend::dag::TokenAction::Burn => {
                        if let Some(target) = &token.target {
                            if target == my_pubkey {
                                incoming_burns.insert(node.id.clone(), node.clone());
                            }
                        }
                    }
                    crate::backend::dag::TokenAction::TransferClaim => {
                        if node.author == my_pubkey {
                            if let Some(ref_cid) = &token.ref_cid {
                                my_claims.insert(ref_cid.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let pending: Vec<DagNode> = incoming_burns
            .into_iter()
            .filter(|(id, _)| !my_claims.contains(id))
            .map(|(_, node)| node)
            .collect();
        Ok(pending)
    }

    pub fn get_balance(&self, pubkey: &str) -> Result<i64, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut balance: i64 = 0;

        for node in nodes {
            if let DagPayload::Token(ref token) = node.payload {
                if node.author == pubkey {
                    match token.action {
                        crate::backend::dag::TokenAction::Mint => balance += token.amount as i64,
                        crate::backend::dag::TokenAction::TransferClaim => balance += token.amount as i64,
                        crate::backend::dag::TokenAction::Burn => balance -= token.amount as i64,
                        _ => {}
                    }
                }
            }
        }
        Ok(balance)
    }

    pub fn count_unique_profiles(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut unique_authors = std::collections::HashSet::new();
        for node in nodes {
            if let DagPayload::Profile(_) = node.payload {
                unique_authors.insert(node.author);
            }
        }
        Ok(unique_authors.len())
    }

    pub fn get_profile(&self, author: &str) -> Result<Option<crate::backend::dag::ProfilePayload>, Box<dyn std::error::Error>> {
        if let Some(head_cid) = self.get_head(author)? {
            let mut current_cid = head_cid;
            loop {
                if let Some(node) = self.get_node(&current_cid)? {
                    if let DagPayload::Profile(p) = node.payload {
                        return Ok(Some(p));
                    }
                    if node.prev.is_empty() {
                        break;
                    }
                    current_cid = node.prev[0].clone();
                } else {
                    break;
                }
            }
        }
        Ok(None)
    }

    pub fn get_last_ubi_claim(&self, pubkey: &str) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut last_claim_time = None;

        for node in nodes {
            if node.author == pubkey {
                if let DagPayload::Token(ref token) = node.payload {
                    if token.action == crate::backend::dag::TokenAction::Mint {
                        if let Some(memo) = &token.memo {
                            if memo == "UBI Daily Claim" {
                                if let Some(current_max) = last_claim_time {
                                    if node.timestamp > current_max {
                                        last_claim_time = Some(node.timestamp);
                                    }
                                } else {
                                    last_claim_time = Some(node.timestamp);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(last_claim_time.map(|t| t.timestamp() as u64))
    }

    pub fn get_proofs(&self, target_pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut proofs = Vec::new();
        for node in nodes {
            if let DagPayload::Proof(ref proof) = node.payload {
                if proof.target_pubkey == target_pubkey {
                    proofs.push(node);
                }
            }
        }
        Ok(proofs)
    }

    pub fn get_web_page(&self, url: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut latest_content: Option<(i64, String)> = None;

        for node in nodes {
            if let DagPayload::Web(ref web) = node.payload {
                if web.url == url {
                    let timestamp = node.timestamp.timestamp();
                    if let Some((latest_ts, _)) = latest_content {
                        if timestamp > latest_ts {
                            latest_content = Some((timestamp, web.content.clone()));
                        }
                    } else {
                        latest_content = Some((timestamp, web.content.clone()));
                    }
                }
            }
        }
        Ok(latest_content.map(|(_, content)| content))
    }

    pub fn get_web_page_node(&self, url: &str) -> Result<Option<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut latest_node: Option<(i64, DagNode)> = None;
        for node in nodes {
            if let DagPayload::Web(ref web) = node.payload {
                if web.url == url {
                    let timestamp = node.timestamp.timestamp();
                    if let Some((latest_ts, _)) = latest_node {
                        if timestamp > latest_ts {
                            latest_node = Some((timestamp, node.clone()));
                        }
                    } else {
                        latest_node = Some((timestamp, node.clone()));
                    }
                }
            }
        }
        Ok(latest_node.map(|(_, node)| node))
    }

    pub fn get_name_record(&self, name: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut latest_record: Option<(i64, String)> = None;
        for node in nodes {
            if let DagPayload::Name(ref record) = node.payload {
                if record.name == name {
                    let timestamp = node.timestamp.timestamp();
                    if let Some((latest_ts, _)) = latest_record {
                        if timestamp > latest_ts {
                            latest_record = Some((timestamp, record.target.clone()));
                        }
                    } else {
                        latest_record = Some((timestamp, record.target.clone()));
                    }
                }
            }
        }
        Ok(latest_record.map(|(_, target)| target))
    }

    /// Returns (block_count, total_bytes_stored)
    pub fn get_storage_stats(&self) -> Result<(usize, usize), Box<dyn std::error::Error>> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let conn = self.conn.lock().unwrap();
            let count: usize = conn.query_row("SELECT COUNT(*) FROM blocks", [], |row| row.get(0))?;
            let total_bytes: usize = conn.query_row("SELECT COALESCE(SUM(LENGTH(data)), 0) FROM blocks", [], |row| row.get(0))?;
            Ok((count, total_bytes))
        }

        #[cfg(target_arch = "wasm32")]
        {
            let blocks = self.blocks.lock().unwrap();
            let count = blocks.len();
            let total_bytes: usize = blocks.values().map(|v| v.len()).sum();
            Ok((count, total_bytes))
        }
    }

    pub fn search_web_pages(&self, query: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let query = query.to_lowercase();
        let mut results = Vec::new();
        // Use a map to keep only the latest version of each page
        let mut latest_pages: std::collections::HashMap<String, (i64, DagNode)> = std::collections::HashMap::new();

        for node in nodes {
             if let DagPayload::Web(ref web) = node.payload {
                let matches = web.title.to_lowercase().contains(&query) 
                    || web.url.to_lowercase().contains(&query)
                    || web.description.to_lowercase().contains(&query)
                    || web.tags.iter().any(|t| t.to_lowercase().contains(&query));
                
                if matches {
                     let timestamp = node.timestamp.timestamp();
                     if let Some((existing_ts, _)) = latest_pages.get(&web.url) {
                         if timestamp > *existing_ts {
                             latest_pages.insert(web.url.clone(), (timestamp, node.clone()));
                         }
                     } else {
                         latest_pages.insert(web.url.clone(), (timestamp, node.clone()));
                     }
                }
            }
        }
        
        for (_, (_, node)) in latest_pages {
            results.push(node);
        }

        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }
    pub fn get_contracts(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut contracts = Vec::new();
        for node in nodes {
             if let DagPayload::Contract(_) = node.payload {
                 contracts.push(node);
             }
        }
        //Sort by timestamp desc
        contracts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(contracts)
    }

    pub fn get_contract_calls(&self, contract_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut calls = Vec::new();
        for node in nodes {
             if let DagPayload::ContractCall(ref call) = node.payload {
                 if call.contract_id == contract_id {
                     calls.push(node);
                 }
             }
        }
        //Sort by timestamp asc (execution order)
        calls.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(calls)
    }

    pub fn get_proposals(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut proposals = Vec::new();
        for node in nodes {
             if let DagPayload::Proposal(_) = node.payload {
                 proposals.push(node);
             }
        }
        proposals.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(proposals)
    }

    pub fn get_votes_for_proposal(&self, proposal_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut votes = Vec::new();
        for node in nodes {
             if let DagPayload::Vote(ref vote) = node.payload {
                 if vote.proposal_id == proposal_id {
                     votes.push(node);
                 }
             }
        }
        votes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(votes)
    }

    /// Get the vote tally for a proposal implementing 1-Human-1-Vote.
    /// Each author's latest vote is the only one that counts.
    /// Returns (yes_count, no_count, abstain_count, petition_count, unique_voters)
    pub fn get_proposal_vote_tally(&self, proposal_id: &str) -> Result<(usize, usize, usize, usize, usize), Box<dyn std::error::Error>> {
        let votes = self.get_votes_for_proposal(proposal_id)?;
        
        // Sort by timestamp descending so latest is first
        let mut sorted_votes = votes;
        sorted_votes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Only keep the latest vote from each author
        let mut latest_votes: std::collections::HashMap<String, crate::backend::dag::VoteType> = std::collections::HashMap::new();
        for node in sorted_votes {
            if let DagPayload::Vote(ref vote) = node.payload {
                // Only insert if author doesn't already have a vote (since we sorted latest first)
                if !latest_votes.contains_key(&node.author) {
                    latest_votes.insert(node.author.clone(), vote.vote.clone());
                }
            }
        }
        
        // Count votes
        let mut yes_count = 0;
        let mut no_count = 0;
        let mut abstain_count = 0;
        let mut petition_count = 0;
        
        for vote_type in latest_votes.values() {
            match vote_type {
                crate::backend::dag::VoteType::Yes => yes_count += 1,
                crate::backend::dag::VoteType::No => no_count += 1,
                crate::backend::dag::VoteType::Abstain => abstain_count += 1,
                crate::backend::dag::VoteType::PetitionSignature => petition_count += 1,
            }
        }
        
        let unique_voters = latest_votes.len();
        Ok((yes_count, no_count, abstain_count, petition_count, unique_voters))
    }

    /// Get all candidacy declarations for a specific ministry
    pub fn get_candidates(&self, ministry: &crate::backend::dag::Ministry) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut candidates = Vec::new();
        for node in nodes {
            if let DagPayload::Candidacy(ref candidacy) = node.payload {
                if &candidacy.ministry == ministry {
                    candidates.push(node);
                }
            }
        }
        candidates.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(candidates)
    }

    /// Get all candidacy declarations across all ministries
    pub fn get_all_candidates(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut candidates = Vec::new();
        for node in nodes {
            if let DagPayload::Candidacy(_) = node.payload {
                candidates.push(node);
            }
        }
        candidates.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(candidates)
    }

    /// Get vote count for a candidate using 1-Human-1-Vote
    /// Each author can only have their latest vote count
    pub fn get_candidate_tally(&self, candidacy_id: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut votes = Vec::new();
        
        // Collect all votes for this candidate
        for node in nodes {
            if let DagPayload::CandidacyVote(ref vote) = node.payload {
                if vote.candidacy_id == candidacy_id {
                    votes.push(node);
                }
            }
        }
        
        // Sort by timestamp descending so latest is first
        votes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Deduplicate by author (1 human = 1 vote)
        let mut unique_voters: std::collections::HashSet<String> = std::collections::HashSet::new();
        for node in votes {
            unique_voters.insert(node.author.clone());
        }
        
        Ok(unique_voters.len())
    }

    pub fn get_my_web_pages(&self, pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut my_pages: std::collections::HashMap<String, (i64, DagNode)> = std::collections::HashMap::new();

        for node in nodes {
             if node.author == pubkey {
                 if let DagPayload::Web(ref web) = node.payload {
                     let timestamp = node.timestamp.timestamp();
                     if let Some((existing_ts, _)) = my_pages.get(&web.url) {
                         if timestamp > *existing_ts {
                             my_pages.insert(web.url.clone(), (timestamp, node.clone()));
                         }
                     } else {
                         my_pages.insert(web.url.clone(), (timestamp, node.clone()));
                     }
                 }
             }
        }

        let mut result = Vec::new();
        for (_, (_, node)) in my_pages {
            result.push(node);
        }
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(result)
    }

    pub fn get_reputation(&self, pubkey: &str) -> Result<crate::backend::dag::ReputationDetails, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        
        let mut verification_score = 0;
        let storage_score = 0; // Placeholder
        let mut content_score = 0;
        let mut governance_score = 0;

        // 1. Identity Score
        if let Ok(Some(profile)) = self.get_profile(pubkey) {
            if profile.founder_id.is_some() {
                verification_score += 100;
            } else {
                // Check if verified (basic check, ideally we use is_verified logic from backend but store doesn't have it easily accessible without recreating logic. 
                // For now, let's assume if they have >0 valid proofs targeting them they are verified.
                // Or we can just check if they are "EligibleForFounder" logic if we want.
                // Let's count incoming proofs.
                 let mut valid_proofs = 0;
                 for node in &nodes {
                      if let DagPayload::Proof(ref p) = node.payload {
                          if p.target_pubkey == pubkey {
                              valid_proofs += 1;
                          }
                      }
                 }
                 if valid_proofs >= 3 { // Simplified verification threshold
                     verification_score += 50; 
                 }
            }
        }

        // 2. Activity Scan
        let mut vouch_count = 0;
        let mut content_count = 0;
        let mut vote_count = 0;

        for node in &nodes {
            if node.author == pubkey {
                match &node.payload {
                    DagPayload::Proof(_) => vouch_count += 1,
                    DagPayload::Post(_) | DagPayload::Web(_) => content_count += 1,
                    DagPayload::Vote(_) => vote_count += 1,
                    _ => {}
                }
            }
        }

        // Cap bonuses
        verification_score += std::cmp::min(vouch_count * 5, 50);
        content_score += std::cmp::min(content_count, 50);
        governance_score += std::cmp::min(vote_count * 2, 50);

        let total_score = verification_score + storage_score + content_score + governance_score;

        Ok(crate::backend::dag::ReputationDetails {
            score: total_score,
            breakdown: crate::backend::dag::ReputationBreakdown {
                verification: verification_score,
                storage: storage_score,
                content: content_score as u32,
                governance: governance_score as u32,
            }
        })
    }

    pub fn get_reports(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut reports = Vec::new();
        for node in nodes {
             if let DagPayload::Report(_) = node.payload {
                 reports.push(node);
             }
        }
        reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(reports)
    }
    pub fn get_my_groups(&self, my_pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut groups = Vec::new();

        for node in nodes {
            if let DagPayload::Group(ref group) = node.payload {
                if group.members.contains(&my_pubkey.to_string()) {
                     groups.push(node);
                }
            }
        }
        groups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(groups)
    }

    pub fn get_group_messages(&self, group_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut messages = Vec::new();
        for node in nodes {
            if let DagPayload::Message(ref msg) = node.payload {
                if let Some(gid) = &msg.group_id {
                    if gid == group_id {
                        messages.push(node);
                    }
                }
            }
        }
        messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        Ok(messages)
    }

    pub fn get_my_files(&self, pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut files = Vec::new();
        for node in nodes {
            if node.author == pubkey {
                if let DagPayload::File(_) = node.payload {
                    files.push(node);
                }
            }
        }
        files.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(files)
    }

    pub fn get_file(&self, cid: &str) -> Result<Option<DagNode>, Box<dyn std::error::Error>> {
        self.get_node(cid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::dag::{DagNode, DagPayload, PostPayload};
    use libp2p::identity::Keypair;

    #[test]
    fn test_store_put_get() {
        let store = Store::new_in_memory().expect("Failed to create store");
        
        let keypair = Keypair::generate_ed25519();
        let payload = DagPayload::Post(PostPayload {
            content: "Hello Store".to_string(),
            attachments: vec![],
            geohash: None,
        });

        let node = DagNode::new(
            "post:v1".to_string(),
            payload,
            vec![],
            &keypair,
            1,
        ).expect("Failed to create node");

        store.put_node(&node).expect("Failed to put node");

        let retrieved = store.get_node(&node.id).expect("Failed to get node");
        assert!(retrieved.is_some());
        
        let retrieved_node = retrieved.unwrap();
        assert_eq!(retrieved_node.id, node.id);
        assert_eq!(retrieved_node.author, node.author);
    }
}
