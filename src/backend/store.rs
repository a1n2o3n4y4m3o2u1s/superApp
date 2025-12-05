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

    /// Get active marketplace listings
    pub fn get_active_listings(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut listings: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if n.r#type == "listing:v1" {
                    if let DagPayload::Listing(ref listing) = n.payload {
                        return listing.status == crate::backend::dag::ListingStatus::Active;
                    }
                }
                false
            })
            .collect();

        listings.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if listings.len() > limit {
            listings.truncate(limit);
        }
        Ok(listings)
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
