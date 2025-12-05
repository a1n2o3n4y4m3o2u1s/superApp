pub mod home_page;
pub mod messaging_page;
pub mod nav_bar;
pub mod profile_page;
pub mod geohash_page;
pub mod verification_page;
pub mod browser_page;
pub mod marketplace_page;

pub use home_page::*;
pub use messaging_page::*;
pub use nav_bar::*;
pub use geohash_page::*;
pub use verification_page::*;
pub use browser_page::*;
pub use marketplace_page::*;

use dioxus::prelude::*;
use std::collections::HashSet;
use crate::backend::dag::DagNode;

#[derive(Clone, Copy)]
pub struct AppState {
    pub messages: Signal<Vec<(DagNode, String)>>,
    pub peers: Signal<HashSet<String>>,
    pub local_peer_id: Signal<String>,
    pub profile: Signal<Option<crate::backend::dag::ProfilePayload>>,
    pub balance: Signal<i64>,
    pub pending_transfers: Signal<Vec<DagNode>>,
    pub geohash: Signal<String>,
    pub ubi_timer: Signal<Option<u64>>,
    pub verification_status: Signal<crate::backend::VerificationStatus>,
    pub viewed_profile: Signal<Option<crate::backend::dag::ProfilePayload>>,
    pub web_content: Signal<Option<String>>,
    pub posts: Signal<Vec<DagNode>>,
    pub blob_cache: Signal<std::collections::HashMap<String, String>>,
    pub last_created_blob: Signal<Option<String>>,
    pub storage_stats: Signal<(usize, usize)>, // (block_count, total_bytes)
    pub local_posts: Signal<Vec<DagNode>>,
    pub listings: Signal<Vec<DagNode>>,
    pub web_search_results: Signal<Vec<DagNode>>,
    pub contracts: Signal<Vec<DagNode>>,
    pub contract_states: Signal<std::collections::HashMap<String, String>>,
}