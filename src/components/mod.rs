pub mod home_page;
pub mod messaging_page;
pub mod nav_bar;
pub mod profile_page;
pub mod geohash_page;
pub mod verification_page;
pub mod browser_page;
pub mod marketplace_page;
pub mod governance_page;










use dioxus::prelude::*;
use std::collections::HashSet;
use crate::backend::dag::DagNode;

#[derive(Clone, Copy)]
pub struct AppState {
    pub peers: Signal<HashSet<String>>,
    pub blocks: Signal<Vec<DagNode>>,
    pub history: Signal<Vec<DagNode>>,
    pub messages: Signal<Vec<(DagNode, String)>>,
    pub groups: Signal<Vec<DagNode>>, // Group definitions
    pub group_messages: Signal<std::collections::HashMap<String, Vec<(DagNode, String)>>>, // GroupId -> Messages
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
    pub blob_cache: Signal<std::collections::HashMap<String, String>>, // CID -> Base64 Data
    pub last_created_blob: Signal<Option<String>>,
    pub storage_stats: Signal<(usize, usize)>, // (block_count, total_bytes)
    pub local_posts: Signal<Vec<DagNode>>,
    pub listings: Signal<Vec<DagNode>>,
    pub web_search_results: Signal<Vec<DagNode>>,
    
    // Contracts
    pub contracts: Signal<Vec<DagNode>>,
    pub contract_states: Signal<std::collections::HashMap<String, String>>, // ContractID -> JSON State
    
    // Governance
    pub proposals: Signal<Vec<DagNode>>,
    pub proposal_votes: Signal<std::collections::HashMap<String, Vec<DagNode>>>,
    pub proposal_tallies: Signal<std::collections::HashMap<String, (usize, usize, usize, usize, usize)>>,
    pub candidates: Signal<Vec<DagNode>>,
    pub candidate_tallies: Signal<std::collections::HashMap<String, usize>>,
    pub reputation: Signal<Option<crate::backend::dag::ReputationDetails>>,
    pub my_web_pages: Signal<Vec<DagNode>>,
    pub user_profiles: Signal<std::collections::HashMap<String, crate::backend::dag::ProfilePayload>>, // Cache
    pub page_title: Signal<String>,
    pub browser_url: Signal<String>,
    pub browser_content: Signal<Option<String>>,
    pub active_tab: Signal<String>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            peers: use_signal(|| HashSet::new()),
            blocks: use_signal(|| vec![]),
            history: use_signal(|| vec![]),
            messages: use_signal(|| vec![]),
            groups: use_signal(|| vec![]),
            group_messages: use_signal(|| std::collections::HashMap::new()),
            local_peer_id: use_signal(|| "Unknown".to_string()),
            profile: use_signal(|| None),
            balance: use_signal(|| 0),
            pending_transfers: use_signal(|| vec![]),
            geohash: use_signal(|| "Global".to_string()),
            ubi_timer: use_signal(|| None),
            verification_status: use_signal(|| crate::backend::VerificationStatus::Unverified),
            viewed_profile: use_signal(|| None),
            web_content: use_signal(|| None),
            posts: use_signal(|| vec![]),
            blob_cache: use_signal(|| std::collections::HashMap::new()),
            last_created_blob: use_signal(|| None),
            storage_stats: use_signal(|| (0, 0)),
            local_posts: use_signal(|| vec![]),
            listings: use_signal(|| vec![]),
            web_search_results: use_signal(|| vec![]),
            contracts: use_signal(|| vec![]),
            contract_states: use_signal(|| std::collections::HashMap::new()),
            proposals: use_signal(|| vec![]),
            proposal_votes: use_signal(|| std::collections::HashMap::new()),
            proposal_tallies: use_signal(|| std::collections::HashMap::new()),
            candidates: use_signal(|| vec![]),
            candidate_tallies: use_signal(|| std::collections::HashMap::new()),
            reputation: use_signal(|| None),
            my_web_pages: use_signal(|| vec![]),
            user_profiles: use_signal(|| std::collections::HashMap::new()),
            page_title: use_signal(|| "SuperApp".to_string()),
            browser_url: use_signal(|| "sp://welcome".to_string()),
            browser_content: use_signal(|| None),
            active_tab: use_signal(|| "feed".to_string()),
        }
    }
}