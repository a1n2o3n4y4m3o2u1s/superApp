#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{params, Connection, Result};
#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};
use std::path::Path;
use crate::backend::dag::{DagNode, DagPayload};
use serde_json;
use chrono::{Utc, Duration};

/// Storage statistics for UI display
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_nodes: usize,
    pub total_bytes: usize,
    #[allow(dead_code)]
    pub nodes_by_type: std::collections::HashMap<String, i64>,
}

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

        // Metadata table for faster indexed queries (avoids parsing all JSON)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks_meta (
                id TEXT PRIMARY KEY,
                author TEXT NOT NULL,
                node_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                target TEXT,
                FOREIGN KEY(id) REFERENCES blocks(id)
            )",
            [],
        )?;

        // Create indexes for common query patterns
        conn.execute("CREATE INDEX IF NOT EXISTS idx_meta_author ON blocks_meta(author)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_meta_type ON blocks_meta(node_type)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_meta_timestamp ON blocks_meta(timestamp)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_meta_target ON blocks_meta(target)", [])?;

        // Separate blob storage table - offloads large binary data from DAG nodes
        conn.execute(
            "CREATE TABLE IF NOT EXISTS blobs (
                cid TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                size INTEGER NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Settings table for storage configuration
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
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
    #[allow(dead_code)]
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

            // Also insert/update metadata for indexed queries
            let node_type = Self::get_node_type(&node.payload);
            let target = Self::get_node_target(&node.payload);
            let timestamp = node.timestamp.timestamp();
            conn.execute(
                "INSERT OR REPLACE INTO blocks_meta (id, author, node_type, timestamp, target) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![node.id, node.author, node_type, timestamp, target],
            )?;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut blocks = self.blocks.lock().unwrap();
            blocks.insert(node.id.clone(), data);
        }

        Ok(())
    }

    /// Extract node type string for metadata indexing
    fn get_node_type(payload: &DagPayload) -> &'static str {
        match payload {
            DagPayload::Profile(_) => "profile",
            DagPayload::Post(_) => "post",
            DagPayload::Proof(_) => "proof",
            DagPayload::Message(_) => "message",
            DagPayload::Group(_) => "group",
            DagPayload::Token(_) => "token",
            DagPayload::Web(_) => "web",
            DagPayload::Name(_) => "name",
            DagPayload::Blob(_) => "blob",
            DagPayload::Listing(_) => "listing",
            DagPayload::Contract(_) => "contract",
            DagPayload::ContractCall(_) => "contract_call",
            DagPayload::Proposal(_) => "proposal",
            DagPayload::Vote(_) => "vote",
            DagPayload::Candidacy(_) => "candidacy",
            DagPayload::CandidacyVote(_) => "candidacy_vote",
            DagPayload::Report(_) => "report",
            DagPayload::File(_) => "file",
            DagPayload::Recall(_) => "recall",
            DagPayload::RecallVote(_) => "recall_vote",
            DagPayload::OversightCase(_) => "oversight_case",
            DagPayload::JuryVote(_) => "jury_vote",
            DagPayload::Comment(_) => "comment",
            DagPayload::Like(_) => "like",
            DagPayload::Story(_) => "story",
            DagPayload::Follow(_) => "follow",
            DagPayload::Course(_) => "course",
            DagPayload::Exam(_) => "exam",
            DagPayload::ExamSubmission(_) => "exam_submission",
            DagPayload::Certification(_) => "certification",
            DagPayload::Application(_) => "application",
            DagPayload::ApplicationVote(_) => "application_vote",
        }
    }

    /// Extract target ID for metadata indexing (recipient, target post, etc.)
    fn get_node_target(payload: &DagPayload) -> Option<String> {
        match payload {
            DagPayload::Message(m) => Some(m.recipient.clone()),
            DagPayload::Proof(p) => Some(p.target_pubkey.clone()),
            DagPayload::Vote(v) => Some(v.proposal_id.clone()),
            DagPayload::Comment(c) => Some(c.parent_id.clone()),
            DagPayload::Like(l) => Some(l.target_id.clone()),
            DagPayload::Follow(f) => Some(f.target.clone()),
            DagPayload::ApplicationVote(av) => Some(av.application_id.clone()),
            DagPayload::Token(t) => t.target.clone(),
            _ => None,
        }
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

    /// Get storage statistics (total nodes, size in bytes by type)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_storage_stats(&self) -> Result<StorageStats, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        
        // Total count and size
        let total_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM blocks",
            [],
            |row| row.get(0),
        )?;
        
        let total_bytes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(data)), 0) FROM blocks",
            [],
            |row| row.get(0),
        )?;

        // Count by type from metadata
        let mut stmt = conn.prepare("SELECT node_type, COUNT(*) FROM blocks_meta GROUP BY node_type")?;
        let type_counts: std::collections::HashMap<String, i64> = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(StorageStats {
            total_nodes: total_count as usize,
            total_bytes: total_bytes as usize,
            nodes_by_type: type_counts,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_storage_stats(&self) -> Result<StorageStats, Box<dyn std::error::Error>> {
        let blocks = self.blocks.lock().unwrap();
        let total_bytes: usize = blocks.values().map(|v| v.len()).sum();
        Ok(StorageStats {
            total_nodes: blocks.len(),
            total_bytes,
            nodes_by_type: std::collections::HashMap::new(),
        })
    }

    /// Prune expired stories (older than 24h) - optional cleanup
    #[cfg(not(target_arch = "wasm32"))]
    pub fn prune_expired_stories(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let cutoff = (Utc::now() - Duration::hours(24)).timestamp();
        
        // Delete from metadata first
        let deleted_count: usize = conn.execute(
            "DELETE FROM blocks_meta WHERE node_type = 'story' AND timestamp < ?1",
            params![cutoff],
        )?;
        
        // Delete orphaned blocks (stories that were pruned)
        conn.execute(
            "DELETE FROM blocks WHERE id IN (
                SELECT b.id FROM blocks b 
                LEFT JOIN blocks_meta m ON b.id = m.id 
                WHERE m.id IS NULL
            )",
            [],
        )?;
        
        Ok(deleted_count)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn prune_expired_stories(&self) -> Result<usize, Box<dyn std::error::Error>> {
        // No-op for wasm32
        Ok(0)
    }

    // =========================================================================
    // BLOB STORAGE METHODS
    // =========================================================================

    /// Store a blob separately from DAG nodes
    #[cfg(not(target_arch = "wasm32"))]
    pub fn put_blob(&self, cid: &str, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().timestamp();
        conn.execute(
            "INSERT OR REPLACE INTO blobs (cid, data, size, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![cid, data, data.len() as i64, now],
        )?;
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn put_blob(&self, _cid: &str, _data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        Ok(()) // No-op for wasm32
    }

    /// Retrieve a blob by CID
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)]
    pub fn get_blob(&self, cid: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT data FROM blobs WHERE cid = ?1")?;
        let result = stmt.query_row(params![cid], |row| row.get::<_, Vec<u8>>(0));
        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_blob(&self, _cid: &str) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        Ok(None) // No-op for wasm32
    }

    /// Get total blob storage size
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_blob_storage_size(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let size: i64 = conn.query_row(
            "SELECT COALESCE(SUM(size), 0) FROM blobs",
            [],
            |row| row.get(0),
        )?;
        Ok(size as usize)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_blob_storage_size(&self) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(0)
    }

    // =========================================================================
    // STORAGE QUOTA METHODS  
    // =========================================================================

    /// Get storage quota in bytes (None = unlimited)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_storage_quota(&self) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        let result: Result<String, _> = conn.query_row(
            "SELECT value FROM settings WHERE key = 'storage_quota_bytes'",
            [],
            |row| row.get(0),
        );
        match result {
            Ok(val) => Ok(val.parse::<u64>().ok()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn get_storage_quota(&self) -> Result<Option<u64>, Box<dyn std::error::Error>> {
        Ok(None)
    }

    /// Set storage quota in bytes (None = unlimited)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_storage_quota(&self, quota_bytes: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.conn.lock().unwrap();
        match quota_bytes {
            Some(bytes) => {
                conn.execute(
                    "INSERT OR REPLACE INTO settings (key, value) VALUES ('storage_quota_bytes', ?1)",
                    params![bytes.to_string()],
                )?;
            }
            None => {
                conn.execute("DELETE FROM settings WHERE key = 'storage_quota_bytes'", [])?;
            }
        }
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_storage_quota(&self, _quota_bytes: Option<u64>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Check storage quota status: (used_bytes, quota_bytes_or_none, usage_percent, is_over_quota)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn check_storage_quota(&self) -> Result<(usize, Option<u64>, u8, bool), Box<dyn std::error::Error>> {
        let stats = self.get_storage_stats()?;
        let blob_size = self.get_blob_storage_size()?;
        let total_used = stats.total_bytes + blob_size;
        
        let quota = self.get_storage_quota()?;
        let (percent, over_quota) = match quota {
            Some(q) if q > 0 => {
                let pct = ((total_used as f64 / q as f64) * 100.0).min(255.0) as u8;
                (pct, total_used > q as usize)
            }
            _ => (0, false),
        };
        
        Ok((total_used, quota, percent, over_quota))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn check_storage_quota(&self) -> Result<(usize, Option<u64>, u8, bool), Box<dyn std::error::Error>> {
        Ok((0, None, 0, false))
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

    #[allow(dead_code)]
    pub fn get_posts_global(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
         let mut posts: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "post:v1")
            .collect();

        posts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(posts)
    }

    pub fn get_recent_stories(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let now = Utc::now();
        let twenty_four_hours_ago = now - Duration::hours(24);

        let mut stories: Vec<DagNode> = nodes.into_iter()
            .filter(|n| {
                if n.r#type == "story:v1" {
                    return n.timestamp > twenty_four_hours_ago;
                }
                false
            })
            .collect();

        stories.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if stories.len() > limit {
            stories.truncate(limit);
        }
        Ok(stories)
    }

    pub fn get_local_stories(&self, geohash_prefix: &str, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let now = Utc::now();
        let twenty_four_hours_ago = now - Duration::hours(24);

        let mut stories: Vec<DagNode> = nodes.into_iter()
            .filter(|n| {
                if n.r#type == "story:v1" && n.timestamp > twenty_four_hours_ago {
                    if let DagPayload::Story(ref story) = n.payload {
                        if let Some(ref gh) = story.geohash {
                            return gh.starts_with(geohash_prefix);
                        }
                    }
                }
                false
            })
            .collect();

        stories.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if stories.len() > limit {
            stories.truncate(limit);
        }
        Ok(stories)
    }

    pub fn get_following(&self, author_pubkey: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut following = std::collections::HashSet::new();

        // Sort by timestamp asc to reconstruct state
        let mut follow_events: Vec<DagNode> = nodes.into_iter()
            .filter(|n| n.author == author_pubkey && n.r#type == "follow:v1")
            .collect();
        follow_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        for node in follow_events {
            if let DagPayload::Follow(f) = node.payload {
                if f.follow {
                    following.insert(f.target);
                } else {
                    following.remove(&f.target);
                }
            }
        }

        Ok(following.into_iter().collect())
    }

    pub fn get_followers(&self, target_pubkey: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        // Map author -> is_following
        let mut follower_map = std::collections::HashMap::new();

        let mut follow_events: Vec<DagNode> = nodes.into_iter()
            .filter(|n| n.r#type == "follow:v1")
            .collect();
        follow_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        for node in follow_events {
            if let DagPayload::Follow(f) = node.payload {
                if f.target == target_pubkey {
                    follower_map.insert(node.author, f.follow);
                }
            }
        }

        let followers: Vec<String> = follower_map.into_iter()
            .filter(|(_, is_following)| *is_following)
            .map(|(author, _)| author)
            .collect();
        Ok(followers)
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

    pub fn get_posts_by_author(&self, author_id: &str, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut posts: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.author == author_id && n.r#type == "post:v1")
            .collect();

        posts.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if posts.len() > limit {
            posts.truncate(limit);
        }
        Ok(posts)
    }

    pub fn get_following_posts(&self, my_pubkey: &str, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let following = self.get_following(my_pubkey)?;
        let following_set: std::collections::HashSet<String> = following.into_iter().collect();
        
        // Also include own posts? Instagram usually does.
        // Let's include own posts too.
        
        let mut posts: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if n.r#type == "post:v1" {
                     return following_set.contains(&n.author) || n.author == my_pubkey;
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

    /// Get active marketplace listings filtered by geohash prefix
    pub fn get_local_listings(&self, geohash_prefix: &str, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut listings: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if n.r#type == "listing:v1" {
                    if let DagPayload::Listing(ref listing) = n.payload {
                        if let Some(ref gh) = listing.geohash {
                            return gh.starts_with(geohash_prefix);
                        }
                    }
                }
                false
            })
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

    /// Count vouches made BY a specific user (not vouches FOR them)
    #[allow(dead_code)]
    pub fn count_vouches_by(&self, author_id: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let count = nodes.iter()
            .filter(|n| {
                n.author == author_id && matches!(n.payload, DagPayload::Proof(_))
            })
            .count();
        Ok(count)
    }

    /// Get timestamp of most recent vouch by a user (for rate limiting)
    #[allow(dead_code)]
    pub fn get_latest_vouch_time(&self, author_id: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let latest = nodes.iter()
            .filter(|n| n.author == author_id && matches!(n.payload, DagPayload::Proof(_)))
            .max_by_key(|n| n.timestamp)
            .map(|n| n.timestamp);
        Ok(latest)
    }

    /// Get profile creation time (for account age check)
    #[allow(dead_code)]
    pub fn get_profile_created_time(&self, author_id: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let earliest = nodes.iter()
            .filter(|n| n.author == author_id && matches!(n.payload, DagPayload::Profile(_)))
            .min_by_key(|n| n.timestamp)
            .map(|n| n.timestamp);
        Ok(earliest)
    }

    /// Get all pending applications (applications without enough approvals)
    pub fn get_pending_applications(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let applications: Vec<DagNode> = nodes.iter()
            .filter(|n| matches!(n.payload, DagPayload::Application(_)))
            .cloned()
            .collect();
        
        // Filter to only include applications that haven't been fully approved yet
        // For simplicity, return all applications - UI will filter based on vote counts
        Ok(applications)
    }

    /// Get all votes for a specific application
    pub fn get_application_votes(&self, application_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let votes: Vec<DagNode> = nodes.iter()
            .filter(|n| {
                if let DagPayload::ApplicationVote(ref av) = n.payload {
                    av.application_id == application_id
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        Ok(votes)
    }

    /// Get timestamp of most recent application vote by a user
    pub fn get_latest_application_vote_time(&self, author_id: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let latest = nodes.iter()
            .filter(|n| n.author == author_id && matches!(n.payload, DagPayload::ApplicationVote(_)))
            .max_by_key(|n| n.timestamp)
            .map(|n| n.timestamp);
        Ok(latest)
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

    #[allow(dead_code)]
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
    pub fn get_public_ledger_events(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        // Retrieve events relevant to the public ledger: Token, Proposal, Vote, Candidacy, Contract, Web
        // We filter by type and sort by timestamp descending
        let nodes = self.get_all_nodes()?;
        let mut events: Vec<DagNode> = nodes.into_iter()
            .filter(|n| {
                matches!(n.r#type.as_str(), 
                    "token:v1" | 
                    "proposal:v1" | 
                    "vote:v1" | 
                    "candidacy:v1" | 
                    "contract:v1" |
                    "contract_call:v1"
                )
            })
            .collect();
        
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(events.into_iter().take(limit).collect())
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

    pub fn get_comments(&self, parent_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let all_nodes = self.get_all_nodes()?;
        let mut comments = Vec::new();

        for node in all_nodes {
            if let DagPayload::Comment(c) = &node.payload {
                if c.parent_id == parent_id {
                    comments.push(node.clone());
                }
            }
        }
        
        // Sort by timestamp (oldest first)
        comments.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(comments)
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

    /// Returns (count, is_liked_by_me)
    pub fn get_likes_for_target(&self, target_id: &str, my_pubkey: &str) -> Result<(usize, bool), Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut likes = Vec::new();

        for node in nodes {
            if let DagPayload::Like(ref like) = node.payload {
                if like.target_id == target_id {
                    likes.push(node);
                }
            }
        }

        // Sort by timestamp descending so latest is first
        likes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Keep latest state per author
        let mut latest_likes: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
        for node in likes {
            if !latest_likes.contains_key(&node.author) {
                if let DagPayload::Like(ref like) = node.payload {
                     // If remove is true, it means they are NOT liking it currently.
                     // But we must track that they interacted.
                     // We actually just want to count those where !remove
                     latest_likes.insert(node.author.clone(), !like.remove);
                }
            }
        }

        let mut count = 0;
        let mut is_liked_by_me = false;

        for (author, active) in latest_likes {
            if active {
                count += 1;
                if author == my_pubkey {
                    is_liked_by_me = true;
                }
            }
        }

        Ok((count, is_liked_by_me))
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
        Ok(result)
    }

    /// Get all web pages from all users, returning only the latest version of each URL
    pub fn get_all_web_pages(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut all_pages: std::collections::HashMap<String, (i64, DagNode)> = std::collections::HashMap::new();

        for node in nodes {
            if let DagPayload::Web(ref web) = node.payload {
                let timestamp = node.timestamp.timestamp();
                if let Some((existing_ts, _)) = all_pages.get(&web.url) {
                    if timestamp > *existing_ts {
                        all_pages.insert(web.url.clone(), (timestamp, node.clone()));
                    }
                } else {
                    all_pages.insert(web.url.clone(), (timestamp, node.clone()));
                }
            }
        }

        let mut result: Vec<DagNode> = all_pages.into_values().map(|(_, n)| n).collect();
        result.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(result)
    }

    pub fn get_proposal_status(&self, proposal_id: &str) -> Result<String, Box<dyn std::error::Error>> {
        let node = self.get_node(proposal_id)?.ok_or("Proposal not found")?;
        let proposal = match node.payload {
            DagPayload::Proposal(p) => p,
            _ => return Err("Node is not a proposal".into()),
        };

        let (yes, no, _abstain, petition, _unique) = self.get_proposal_vote_tally(proposal_id)?;
        let total_users = self.count_unique_profiles()?;
        
        // Avoid division by zero
        let total_users = if total_users == 0 { 1 } else { total_users };

        // Thresholds
        let (petition_threshold_percent, voting_duration_hours, pass_threshold_percent) = match proposal.r#type {
            crate::backend::dag::ProposalType::Standard => (0.01, 168, 0.50), // 1% sigs, 1 week, >50% yes
            crate::backend::dag::ProposalType::Constitutional => (0.01, 168, 0.66), // 1% sigs, 1 week, >66% yes
            crate::backend::dag::ProposalType::Emergency => (0.05, 48, 0.50), // 5% sigs, 48 hours, >50% yes
            crate::backend::dag::ProposalType::SetTax(_) => (0.01, 168, 0.50), // Treat as Standard for now
            crate::backend::dag::ProposalType::DefineMinistries(_) => (0.01, 168, 0.50), // Standard requirements
        };

        let petition_threshold = (total_users as f64 * petition_threshold_percent).ceil() as usize;
        
        // Check if in Petition Phase
        // meaningful_votes includes Petition signatures AND Yes votes (implicit support)
        let meaningful_votes = petition + yes; 
        if meaningful_votes < petition_threshold {
            return Ok(format!("Petitioning ({}/{})", meaningful_votes, petition_threshold));
        }

        // Voting Phase
        let now = chrono::Utc::now();
        let created_at = node.timestamp;
        let duration = now.signed_duration_since(created_at);
        let hours_elapsed = duration.num_hours();

        if hours_elapsed < voting_duration_hours {
             let hours_left = voting_duration_hours - hours_elapsed;
             return Ok(format!("Voting ({}h left)", hours_left));
        }

        // Finished - Calculate Result
        let total_votes = yes + no; // Abstains don't count towards denominator in simple majority usually, or do they? 
        // Plan says "Simple majority (50%+1)" and "Supermajority (66%+)". Usually implies of votes cast.
        
        if total_votes == 0 {
            return Ok("Failed (No votes)".to_string());
        }

        let yes_percent = yes as f64 / total_votes as f64;
        
        if yes_percent > pass_threshold_percent {
            Ok("Passed".to_string())
        } else {
            Ok("Rejected".to_string())
        }
    }


    /// Get the current list of elected officials (Ministry -> Pubkey)
    pub fn get_active_officials(&self) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        let ministries = self.get_active_ministries()?;
        let mut officials = std::collections::HashMap::new();

        for ministry in ministries {
            // Get candidates for this ministry
            let candidates = self.get_candidates(&ministry)?;
            
            // Tally votes
            let mut best_candidate: Option<(String, usize)> = None;
            
            for candidate_node in candidates {
                if let DagPayload::Candidacy(_) = candidate_node.payload {
                    let votes = self.get_candidate_tally(&candidate_node.id)?;
                    
                    if votes > 0 {
                        match best_candidate {
                            Some((_, max_votes)) => {
                                if votes > max_votes {
                                    best_candidate = Some((candidate_node.author.clone(), votes));
                                }
                            }
                            None => {
                                best_candidate = Some((candidate_node.author.clone(), votes));
                            }
                        }
                    }
                }
            }

            if let Some((winner_pubkey, _)) = best_candidate {
                officials.insert(ministry, winner_pubkey);
            }
        }

        Ok(officials)
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

        // 3. Elected Official Bonus
        if let Ok(officials) = self.get_active_officials() {
            if officials.values().any(|p| p == pubkey) {
                governance_score += 100; // Big bonus for being an elected official
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

    #[allow(dead_code)]
    pub fn get_file(&self, cid: &str) -> Result<Option<DagNode>, Box<dyn std::error::Error>> {
        self.get_node(cid)
    }

    pub fn get_recalls(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut recalls = Vec::new();
        for node in nodes {
             if let DagPayload::Recall(_) = node.payload {
                 recalls.push(node);
             }
        }
        recalls.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(recalls)
    }

    pub fn get_recall_votes(&self, recall_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut votes = Vec::new();
        for node in nodes {
             if let DagPayload::RecallVote(ref vote) = node.payload {
                 if vote.recall_id == recall_id {
                     votes.push(node);
                 }
             }
        }
        votes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(votes)
    }

    pub fn get_recall_tally(&self, recall_id: &str) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
        let votes = self.get_recall_votes(recall_id)?;
        
        // Sort by timestamp descending so latest is first
        let mut sorted_votes = votes;
        sorted_votes.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Only keep the latest vote from each author
        let mut latest_votes: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
        for node in sorted_votes {
            if let DagPayload::RecallVote(ref vote) = node.payload {
                if !latest_votes.contains_key(&node.author) {
                    latest_votes.insert(node.author.clone(), vote.vote);
                }
            }
        }
        
        let mut remove_count = 0;
        let mut keep_count = 0;
        
        for remove in latest_votes.values() {
            if *remove {
                remove_count += 1;
            } else {
                keep_count += 1;
            }
        }
        
        let unique_voters = latest_votes.len();
        Ok((remove_count, keep_count, unique_voters))
    }

    pub fn get_oversight_cases(&self) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut cases = Vec::new();
        for node in nodes {
             if let DagPayload::OversightCase(_) = node.payload {
                 cases.push(node);
             }
        }
        cases.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(cases)
    }

    pub fn get_user_jury_duty(&self, user_pubkey: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_oversight_cases()?;
        let mut duty = Vec::new();
        for node in nodes {
            if let DagPayload::OversightCase(ref case) = node.payload {
                if case.jury_members.contains(&user_pubkey.to_string()) && case.status == "Open" {
                    duty.push(node);
                }
            }
        }
        Ok(duty)
    }

    #[allow(dead_code)]
    pub fn get_jury_votes(&self, case_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut votes = Vec::new();
        for node in nodes {
             if let DagPayload::JuryVote(ref vote) = node.payload {
                 if vote.case_id == case_id {
                     votes.push(node);
                 }
             }
        }
        Ok(votes)
    }

    pub fn get_current_tax_rate(&self) -> Result<u8, Box<dyn std::error::Error>> {
        let proposals = self.get_proposals()?;
        
        // Filter for SetTax proposals
        let mut tax_proposals = Vec::new();
        for node in proposals {
            if let DagPayload::Proposal(ref p) = node.payload {
                if let crate::backend::dag::ProposalType::SetTax(rate) = p.r#type {
                    tax_proposals.push((node.id.clone(), rate, node.timestamp));
                }
            }
        }

        // Sort by timestamp descending (latest first)
        tax_proposals.sort_by(|a, b| b.2.cmp(&a.2));

        // Find the first one that passed
        for (id, rate, _) in tax_proposals {
             if let Ok(status) = self.get_proposal_status(&id) {
                 if status == "Passed" {
                     return Ok(rate);
                 }
             }
        }

        // Default to 0 if no tax proposal has passed
        Ok(0)
    }
    pub fn search_files(&self, query: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        for node in nodes {
             if let DagPayload::File(ref f) = node.payload {
                let matches = f.name.to_lowercase().contains(&query_lower) 
                    || f.mime_type.to_lowercase().contains(&query_lower);
                
                if matches {
                     results.push(node);
                }
            }
        }
        
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
    }
    pub fn get_active_ministries(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // Start with the default ministries
        let default_ministries = vec![
            "VerificationAndIdentity".to_string(),
            "TreasuryAndDistribution".to_string(),
            "NetworkAndProtocols".to_string(),
        ];

        // Scan for passed "DefineMinistries" proposals to override this list
        let proposals = self.get_proposals()?;
        
        // Filter for DefineMinistries proposals
        let mut ministry_proposals = Vec::new();
        for node in proposals {
            if let DagPayload::Proposal(ref p) = node.payload {
                if let crate::backend::dag::ProposalType::DefineMinistries(ref m) = p.r#type {
                    ministry_proposals.push((node.id.clone(), m.clone(), node.timestamp));
                }
            }
        }

        // Sort by timestamp descending (latest first)
        ministry_proposals.sort_by(|a, b| b.2.cmp(&a.2));

        // Find the first one that passed
        for (id, ministries, _) in ministry_proposals {
             if let Ok(status) = self.get_proposal_status(&id) {
                 if status == "Passed" {
                     return Ok(ministries);
                 }
             }
        }
        
        Ok(default_ministries)
    }

    // ======= EDUCATION SYSTEM =======

    /// Get all courses, sorted by timestamp (most recent first)
    pub fn get_courses(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut courses: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "course:v1")
            .collect();

        courses.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if courses.len() > limit {
            courses.truncate(limit);
        }
        Ok(courses)
    }

    /// Get all exams, sorted by timestamp (most recent first)
    pub fn get_exams(&self, limit: usize) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let mut exams: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.r#type == "exam:v1")
            .collect();

        exams.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        if exams.len() > limit {
            exams.truncate(limit);
        }
        Ok(exams)
    }

    /// Get all certifications for a specific user
    pub fn get_certifications(&self, peer_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let certifications: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| {
                if n.r#type == "certification:v1" {
                    if let DagPayload::Certification(ref cert) = n.payload {
                        return cert.recipient == peer_id;
                    }
                }
                false
            })
            .collect();
        Ok(certifications)
    }

    /// Get all exam submissions by a specific user
    #[allow(dead_code)]
    pub fn get_exam_submissions(&self, peer_id: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let submissions: Vec<DagNode> = self.get_all_nodes()?
            .into_iter()
            .filter(|n| n.author == peer_id && n.r#type == "exam_submission:v1")
            .collect();
        Ok(submissions)
    }
    
    // ======= SMART CONTRACTS =======
    
    /// Get all nodes that reference a specific CID. 
    /// Useful for getting finding Contract Calls and Token Transfers related to a contract.
    pub fn get_nodes_by_ref(&self, ref_cid: &str) -> Result<Vec<DagNode>, Box<dyn std::error::Error>> {
        let nodes = self.get_all_nodes()?;
        let mut results = Vec::new();
        
        for node in nodes {
            match &node.payload {
                DagPayload::ContractCall(call) => {
                    if call.contract_id == ref_cid {
                        results.push(node.clone());
                    }
                }
                DagPayload::Token(token) => {
                    if let Some(r_cid) = &token.ref_cid {
                        if r_cid == ref_cid {
                             results.push(node.clone());
                        }
                    }
                }
                DagPayload::Listing(listing) => {
                    if let Some(r_cid) = &listing.ref_cid {
                        if r_cid == ref_cid {
                            results.push(node.clone());
                        }
                    }
                }
                _ => {}
            }
        }
        
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(results)
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
        // author in DagNode is now PeerID string
        assert_eq!(retrieved_node.author, libp2p::PeerId::from_public_key(&keypair.public()).to_string());
    }
    #[test]
    fn test_tax_rate_logic() {
        use chrono::{Duration, Utc};
        let store = Store::new_in_memory().expect("Failed to create store");
        let keypair = Keypair::generate_ed25519();
        let pubkey_hex = hex::encode(keypair.public().encode_protobuf());

        // 1. Initial rate should be 0
        assert_eq!(store.get_current_tax_rate().unwrap(), 0);

        // 2. Setup: Make user a founder so they count as "Total Users" (1) and are Verified
        let profile_payload = DagPayload::Profile(crate::backend::dag::ProfilePayload {
            name: "Founder".to_string(),
            bio: "".to_string(),
            founder_id: Some(1),
            encryption_pubkey: None,
            photo: None,
        });
        let profile = DagNode::new("profile:v1".to_string(), profile_payload, vec![], &keypair, 0).unwrap();
        store.put_node(&profile).expect("Failed to store profile");
        store.update_head(&pubkey_hex, &profile.id).expect("Failed updates");

        // 3. Create a Tax Proposal (10%) - Need it to be "Passed"
        // Condition: >50% yes AND Duration passed (1 week for Standard/SetTax)
        let payload = DagPayload::Proposal(crate::backend::dag::ProposalPayload {
            title: "Tax 10%".to_string(),
            description: "impot".to_string(),
            r#type: crate::backend::dag::ProposalType::SetTax(10),
        });
        
        let mut proposal = DagNode::new("proposal:v1".to_string(), payload, vec![], &keypair, 0).unwrap();
        
        // Manipulate timestamp to 8 days ago
        proposal.timestamp = Utc::now() - Duration::days(8);
        proposal.id = proposal.calculate_cid().unwrap();
        proposal.sig = proposal.sign(&keypair).unwrap();
        
        store.put_node(&proposal).unwrap();

        // Still 0 because no votes
        assert_eq!(store.get_current_tax_rate().unwrap(), 0);

        // 4. Vote Yes
        let vote_payload = DagPayload::Vote(crate::backend::dag::VotePayload {
            proposal_id: proposal.id.clone(),
            vote: crate::backend::dag::VoteType::Yes,
        });
        let vote = DagNode::new("vote:v1".to_string(), vote_payload, vec![], &keypair, 0).unwrap();
        store.put_node(&vote).unwrap();

        // Should be 10 now (1/1 users = 100% > 50%, Time > 1 week)
        assert_eq!(store.get_current_tax_rate().unwrap(), 10);

        // 5. Create a newer proposal (20%) - Recent (not passed time)
        let payload2 = DagPayload::Proposal(crate::backend::dag::ProposalPayload {
            title: "Tax 20%".to_string(),
            description: "more".to_string(),
            r#type: crate::backend::dag::ProposalType::SetTax(20),
        });
        let proposal2 = DagNode::new("proposal:v1".to_string(), payload2, vec![], &keypair, 0).unwrap();
        store.put_node(&proposal2).unwrap();
        
        let vote2_payload = DagPayload::Vote(crate::backend::dag::VotePayload {
            proposal_id: proposal2.id.clone(),
            vote: crate::backend::dag::VoteType::Yes,
        });
        let vote2 = DagNode::new("vote:v1".to_string(), vote2_payload, vec![], &keypair, 0).unwrap();
        store.put_node(&vote2).unwrap();

        // Still 10 because 20% proposal is "Voting" (time not elapsed)
        assert_eq!(store.get_current_tax_rate().unwrap(), 10);
    }

    #[test]
    fn test_dynamic_ministries() {
        use chrono::{Duration, Utc};
        let store = Store::new_in_memory().expect("Failed to create store");
        let keypair = Keypair::generate_ed25519();
        let pubkey_hex = hex::encode(keypair.public().encode_protobuf());

        // 1. Initial State: Default Ministries
        let defaults = store.get_active_ministries().unwrap();
        assert_eq!(defaults.len(), 3);
        assert!(defaults.contains(&"VerificationAndIdentity".to_string()));

        // 2. Setup User (Founder)
        let profile_payload = DagPayload::Profile(crate::backend::dag::ProfilePayload {
            name: "Founder".to_string(),
            bio: "".to_string(),
            founder_id: Some(1),
            encryption_pubkey: None,
            photo: None,
        });
        let profile = DagNode::new("profile:v1".to_string(), profile_payload, vec![], &keypair, 0).unwrap();
        store.put_node(&profile).expect("Failed to store profile");
        store.update_head(&pubkey_hex, &profile.id).expect("Failed updates");

        // 3. Create Proposal to Change Ministries
        let new_ministries = vec!["MinistryOfTruth".to_string(), "MinistryOfPeace".to_string()];
        let payload = DagPayload::Proposal(crate::backend::dag::ProposalPayload {
            title: "New World Order".to_string(),
            description: "Better ministries".to_string(),
            r#type: crate::backend::dag::ProposalType::DefineMinistries(new_ministries.clone()),
        });
        
        let mut proposal = DagNode::new("proposal:v1".to_string(), payload, vec![], &keypair, 0).unwrap();
        // Manipulate time to ensure it can pass
        proposal.timestamp = Utc::now() - Duration::days(8);
        proposal.id = proposal.calculate_cid().unwrap(); 
        proposal.sig = proposal.sign(&keypair).unwrap();
        
        store.put_node(&proposal).unwrap();

        // 4. Vote Yes to pass it
        let vote_payload = DagPayload::Vote(crate::backend::dag::VotePayload {
            proposal_id: proposal.id.clone(),
            vote: crate::backend::dag::VoteType::Yes,
        });
        let vote = DagNode::new("vote:v1".to_string(), vote_payload, vec![], &keypair, 0).unwrap();
        store.put_node(&vote).unwrap();

        // 5. Check Ministries - Should be new list
        let active = store.get_active_ministries().unwrap();
        assert_eq!(active.len(), 2);
        assert_eq!(active, new_ministries);

        // 6. Create NEWER proposal but DON'T vote on it (Status: Voting)
        let newer_ministries = vec!["MinistryOfSillyWalks".to_string()];
        let payload2 = DagPayload::Proposal(crate::backend::dag::ProposalPayload {
            title: "Silly".to_string(),
            description: "Walking".to_string(),
            r#type: crate::backend::dag::ProposalType::DefineMinistries(newer_ministries.clone()),
        });
        let proposal2 = DagNode::new("proposal:v1".to_string(), payload2, vec![], &keypair, 0).unwrap();
        store.put_node(&proposal2).unwrap();

        // Should still be the previous passed one
        let active_now = store.get_active_ministries().unwrap();
        assert_eq!(active_now, new_ministries);
    }
}


    #[test]
    fn test_social_comments() {
        let store = Store::new_in_memory().unwrap();
        
        // 1. Create a Post
        let post_payload = DagPayload::Post(crate::backend::dag::PostPayload {
             content: "Main Post".to_string(),
             attachments: vec![],
             geohash: None,
        });
        let post = crate::backend::dag::DagNode::new(
             "post:v1".to_string(),
             post_payload,
             vec![],
             &crate::backend::dag::Keypair::generate_ed25519(),
             0,
        ).unwrap();
        store.put_node(&post).unwrap();

        // 2. Create a Comment
        let comment_payload = DagPayload::Comment(crate::backend::dag::CommentPayload {
            parent_id: post.id.clone(),
            content: "First Comment".to_string(),
            attachments: vec![],
        });
        let comment = crate::backend::dag::DagNode::new(
             "comment:v1".to_string(),
             comment_payload,
             vec![],
             &crate::backend::dag::Keypair::generate_ed25519(),
             0,
        ).unwrap();
        store.put_node(&comment).unwrap();

        // 3. Verify
        let comments = store.get_comments(&post.id).unwrap();
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, comment.id);
        
        if let DagPayload::Comment(c) = &comments[0].payload {
            assert_eq!(c.content, "First Comment");
        } else {
             panic!("Wrong payload type");
        }
    }
