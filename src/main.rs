mod backend;
mod components;

use components::nav_bar;
use nav_bar::NavComponent;
use components::verification_page;
use verification_page::VerificationPage;
use components::superweb_shell;
use superweb_shell::SuperWebShell;
use components::smart_contracts_page;
use smart_contracts_page::SmartContractsPage;

use dioxus::prelude::*;
use tokio::sync::mpsc;
use backend::{AppCmd, AppEvent};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(RootLayout)]
        #[layout(NavComponent)]
        #[route("/")]
        SuperWebShell {},

        #[end_layout]
    
    #[route("/verify")]
    VerificationPage {},
    
}

use std::collections::HashSet;
use backend::dag::DagNode;
use components::AppState;

/// Struct to hold all mutable signals for event handling
struct EventSignals {
    messages: Signal<Vec<(DagNode, String)>>,
    peers: Signal<HashSet<String>>,
    local_peer_id: Signal<String>,
    profile: Signal<Option<backend::dag::ProfilePayload>>,
    balance: Signal<i64>,
    pending_transfers: Signal<Vec<DagNode>>,
    geohash: Signal<String>,
    ubi_timer: Signal<Option<u64>>,
    verification_status: Signal<backend::VerificationStatus>,
    viewed_profile: Signal<Option<backend::dag::ProfilePayload>>,
    web_content: Signal<Option<String>>,
    posts: Signal<Vec<DagNode>>,
    blob_cache: Signal<std::collections::HashMap<String, String>>,
    last_created_blob: Signal<Option<String>>,
    storage_stats: Signal<(usize, usize)>,
    local_posts: Signal<Vec<DagNode>>,
    listings: Signal<Vec<DagNode>>,
    local_listings: Signal<Vec<DagNode>>,
    web_search_results: Signal<Vec<DagNode>>,
    contracts: Signal<Vec<DagNode>>,
    active_contract_history: Signal<Vec<DagNode>>,
    pending_contracts: Signal<Vec<DagNode>>,
    contract_states: Signal<std::collections::HashMap<String, String>>,
    proposals: Signal<Vec<DagNode>>,
    proposal_votes: Signal<std::collections::HashMap<String, Vec<DagNode>>>,
    proposal_tallies: Signal<std::collections::HashMap<String, (usize, usize, usize, usize, usize, String)>>,
    current_tax_rate: Signal<u8>,
    candidates: Signal<Vec<DagNode>>,
    candidate_tallies: Signal<std::collections::HashMap<String, usize>>,
    recalls: Signal<Vec<DagNode>>,
    recall_tallies: Signal<std::collections::HashMap<String, (usize, usize, usize)>>,
    oversight_cases: Signal<Vec<DagNode>>,
    jury_duty: Signal<Vec<DagNode>>,
    reputation: Signal<Option<backend::dag::ReputationDetails>>,
    my_web_pages: Signal<Vec<DagNode>>,
    all_web_pages: Signal<Vec<DagNode>>,
    groups: Signal<Vec<DagNode>>,
    group_messages: Signal<std::collections::HashMap<String, Vec<(DagNode, String)>>>,
    reports: Signal<Vec<DagNode>>,
    files: Signal<Vec<DagNode>>,
    public_ledger: Signal<Vec<DagNode>>,
    file_search_results: Signal<Vec<DagNode>>,
    ministries: Signal<Vec<String>>,
    comments: Signal<std::collections::HashMap<String, Vec<DagNode>>>,
    likes: Signal<std::collections::HashMap<String, (usize, bool)>>,
    stories: Signal<Vec<DagNode>>,
    local_stories: Signal<Vec<DagNode>>,
    seen_stories: Signal<HashSet<String>>,
    following: Signal<Vec<String>>,
    user_posts: Signal<Vec<DagNode>>,
    following_posts: Signal<Vec<DagNode>>,
    courses: Signal<Vec<DagNode>>,
    exams: Signal<Vec<DagNode>>,
    certifications: Signal<Vec<DagNode>>,
    pending_applications: Signal<Vec<DagNode>>,
    exam_result: Signal<Option<(String, u8, bool)>>,
}

fn handle_app_event(event: AppEvent, sigs: &mut EventSignals, cmd_tx: &tokio::sync::mpsc::UnboundedSender<AppCmd>) {
    match event {
        AppEvent::MessageReceived(node, content) => {
            let author = node.author.clone();
            if !sigs.peers.read().contains(&author) {
                sigs.peers.write().insert(author);
            }
            sigs.messages.write().push((node, content));
        }
        AppEvent::MessagesFetched(msgs) => {
            let mut peers_write = sigs.peers.write();
            for (msg, _) in &msgs {
                if !peers_write.contains(&msg.author) {
                    peers_write.insert(msg.author.clone());
                }
            }
            drop(peers_write);
            *sigs.messages.write() = msgs;
        }
        AppEvent::PeerDiscovered(peer) => {
            sigs.peers.write().insert(peer);
        }
        AppEvent::MyIdentity(id) => {
            sigs.local_peer_id.set(id.clone());
            let _ = cmd_tx.send(AppCmd::FetchFollowing { target: id });
        }
        AppEvent::ProfileFetched(p) => {
            sigs.profile.set(p);
        }
        AppEvent::BalanceFetched(bal) => {
            sigs.balance.set(bal);
        }
        AppEvent::PendingTransfersFetched(nodes) => {
            sigs.pending_transfers.set(nodes);
        }
        AppEvent::GeohashDetected(hash) => {
            sigs.geohash.set(hash);
        }
        AppEvent::UbiTimerFetched(time) => {
            sigs.ubi_timer.set(time);
        }
        AppEvent::VerificationStatus(status) => {
            println!("Verification status update: {:?}", status);
            sigs.verification_status.set(status.clone());
        }
        AppEvent::UserProfileFetched(p) => {
            sigs.viewed_profile.set(p);
        }
        AppEvent::WebPageFetched { url, content } => {
            if let Some(c) = content {
                sigs.web_content.set(Some(c));
            } else {
                sigs.web_content.set(Some(format!("<h1>404 Not Found</h1><p>Could not find page: {}</p>", url)));
            }
        }
        AppEvent::HistoryFetched(fetched_posts) => {
            sigs.posts.set(fetched_posts);
        }
        AppEvent::BlockReceived(node) => {
            match node.r#type.as_str() {
                "post:v1" => {
                    sigs.posts.write().insert(0, node.clone());
                }
                "blob:v1" => {
                    if let backend::dag::DagPayload::Blob(blob) = &node.payload {
                        sigs.blob_cache.write().insert(node.id.clone(), format!("data:{};base64,{}", blob.mime_type, blob.data));
                        if node.author == sigs.local_peer_id.read().clone() {
                            sigs.last_created_blob.set(Some(node.id.clone()));
                        }
                    }
                }
                "proposal:v1" => {
                    sigs.proposals.write().insert(0, node.clone());
                }
                "vote:v1" => {
                    if let backend::dag::DagPayload::Vote(v) = &node.payload {
                        let mut votes_map = sigs.proposal_votes.write();
                        let entry = votes_map.entry(v.proposal_id.clone()).or_insert(Vec::new());
                        entry.push(node.clone());
                    }
                }
                "candidacy:v1" => {
                    sigs.candidates.write().insert(0, node.clone());
                }
                "candidacy_vote:v1" => {}
                "file:v1" => {
                    sigs.files.write().insert(0, node.clone());
                }
                "story:v1" => {
                    sigs.stories.write().insert(0, node.clone());
                }
                "follow:v1" => {
                    if node.author == sigs.local_peer_id.read().clone() {
                        if let backend::dag::DagPayload::Follow(f) = &node.payload {
                            if f.follow {
                                if !sigs.following.read().contains(&f.target) {
                                    sigs.following.write().push(f.target.clone());
                                }
                            } else {
                                sigs.following.write().retain(|x| x != &f.target);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        AppEvent::BlockFetched { cid: _, node } => {
            if let Some(n) = node {
                if n.r#type == "blob:v1" {
                    if let backend::dag::DagPayload::Blob(blob) = &n.payload {
                        sigs.blob_cache.write().insert(n.id.clone(), format!("data:{};base64,{}", blob.mime_type, blob.data));
                    }
                }
            }
        }
        AppEvent::StorageStatsFetched { block_count, total_bytes } => {
            sigs.storage_stats.set((block_count, total_bytes));
        }
        AppEvent::LocalPostsFetched(fetched_posts) => {
            sigs.local_posts.set(fetched_posts);
        }
        AppEvent::ListingsFetched(fetched_listings) => {
            sigs.listings.set(fetched_listings);
        }
        AppEvent::LocalListingsFetched(fetched_listings) => {
            sigs.local_listings.set(fetched_listings);
        }
        AppEvent::WebSearchResults(results) => {
            sigs.web_search_results.set(results);
        }
        AppEvent::ContractsFetched(fetched_contracts) => {
            sigs.contracts.set(fetched_contracts);
        }
        AppEvent::ContractStateFetched { contract_id, state } => {
            sigs.contract_states.write().insert(contract_id, state);
        }
        AppEvent::ContractHistoryFetched { contract_id: _, history } => {
            sigs.active_contract_history.set(history);
        }
        AppEvent::PendingContractsFetched(contracts) => {
            sigs.pending_contracts.set(contracts);
        }
        AppEvent::ProposalsFetched(fetched_proposals) => {
            sigs.proposals.set(fetched_proposals);
        }
        AppEvent::ProposalVotesFetched { proposal_id, votes } => {
            sigs.proposal_votes.write().insert(proposal_id, votes);
        }
        AppEvent::ProposalTallyFetched { proposal_id, yes, no, abstain, petition, unique_voters, status } => {
            sigs.proposal_tallies.write().insert(proposal_id, (yes, no, abstain, petition, unique_voters, status));
        }
        AppEvent::TaxRateFetched(rate) => {
            sigs.current_tax_rate.set(rate);
        }
        AppEvent::CandidatesFetched(fetched_candidates) => {
            sigs.candidates.set(fetched_candidates);
        }
        AppEvent::CandidateTallyFetched { candidacy_id, votes } => {
            sigs.candidate_tallies.write().insert(candidacy_id, votes);
        }
        AppEvent::RecallsFetched(fetched_recalls) => {
            sigs.recalls.set(fetched_recalls);
        }
        AppEvent::RecallTallyFetched { recall_id, remove, keep, unique_voters } => {
            sigs.recall_tallies.write().insert(recall_id, (remove, keep, unique_voters));
        }
        AppEvent::ReputationFetched(details) => {
            sigs.reputation.set(Some(details));
        }
        AppEvent::OversightCasesFetched(cases) => {
            sigs.oversight_cases.set(cases);
        }
        AppEvent::JuryDutyFetched(cases) => {
            sigs.jury_duty.set(cases);
        }
        AppEvent::MyWebPagesFetched(pages) => {
            sigs.my_web_pages.set(pages);
        }
        AppEvent::AllWebPagesFetched(pages) => {
            sigs.all_web_pages.set(pages);
        }
        AppEvent::StoriesFetched(fetched_stories) => {
            sigs.stories.set(fetched_stories);
        }
        AppEvent::LocalStoriesFetched(fetched_stories) => {
            sigs.local_stories.set(fetched_stories);
        }
        AppEvent::GroupMessagesFetched(msgs) => {
            if !msgs.is_empty() {
                if let backend::dag::DagPayload::Message(p) = &msgs[0].0.payload {
                    if let Some(gid) = &p.group_id {
                        sigs.group_messages.write().insert(gid.clone(), msgs);
                    }
                }
            }
        }
        AppEvent::ReportsFetched(fetched_reports) => {
            sigs.reports.set(fetched_reports);
        }
        AppEvent::MyFilesFetched(fetched_files) => {
            sigs.files.set(fetched_files);
        }
        AppEvent::FileUploaded(_node) => {}
        AppEvent::PublicLedgerFetched(events) => {
            sigs.public_ledger.set(events);
        }
        AppEvent::FileSearchResults(results) => {
            sigs.file_search_results.set(results);
        }
        AppEvent::MinistriesFetched(m) => {
            sigs.ministries.set(m);
        }
        AppEvent::CommentsFetched { parent_id, comments: c } => {
            sigs.comments.write().insert(parent_id, c);
        }
        AppEvent::LikesFetched { target_id, count, is_liked_by_me } => {
            sigs.likes.write().insert(target_id, (count, is_liked_by_me));
        }
        AppEvent::FollowingFetched(f) => {
            sigs.following.set(f);
        }
        AppEvent::UserPostsFetched(p) => {
            sigs.user_posts.set(p);
        }
        AppEvent::FollowingPostsFetched(p) => {
            sigs.following_posts.set(p);
        }
        AppEvent::CoursesFetched(c) => {
            sigs.courses.set(c);
        }
        AppEvent::ExamsFetched(e) => {
            sigs.exams.set(e);
        }
        AppEvent::CertificationsFetched(c) => {
            sigs.certifications.set(c);
        }
        AppEvent::ExamSubmitted { exam_id, score, passed } => {
            println!("Exam {} submitted: score={}, passed={}", exam_id, score, passed);
            sigs.exam_result.set(Some((exam_id, score, passed)));
        }
        AppEvent::PendingApplicationsFetched(apps) => {
            sigs.pending_applications.set(apps);
        }
        AppEvent::ApplicationVotesFetched { .. } => {}
        _ => {}
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize global state
    let messages = use_signal(|| Vec::<(DagNode, String)>::new());
    let blocks = use_signal(|| Vec::<DagNode>::new());
    let history = use_signal(|| Vec::<DagNode>::new());
    let user_profiles = use_signal(|| std::collections::HashMap::<String, backend::dag::ProfilePayload>::new());
    let page_title = use_signal(|| "SuperApp".to_string());
    let browser_url = use_signal(|| "sp://welcome".to_string());
    let browser_content = use_signal(|| None::<String>);
    let active_tab = use_signal(|| "feed".to_string());
    let peers = use_signal(|| HashSet::<String>::new());
    let local_peer_id = use_signal(|| String::new());
    let profile = use_signal(|| None::<backend::dag::ProfilePayload>);
    let balance = use_signal(|| 0i64);
    let pending_transfers = use_signal(|| Vec::<DagNode>::new());
    let geohash = use_signal(|| "Global".to_string());
    let ubi_timer = use_signal(|| None::<u64>);
    let verification_status = use_signal(|| backend::VerificationStatus::Unverified);
    let viewed_profile = use_signal(|| None::<backend::dag::ProfilePayload>);
    let web_content = use_signal(|| None::<String>);
    let posts = use_signal(|| Vec::<DagNode>::new());
    let blob_cache = use_signal(|| std::collections::HashMap::<String, String>::new());
    let last_created_blob = use_signal(|| None::<String>);
    let storage_stats = use_signal(|| (0usize, 0usize));
    let local_posts = use_signal(|| Vec::<DagNode>::new());
    let listings = use_signal(|| Vec::<DagNode>::new());
    let local_listings = use_signal(|| Vec::<DagNode>::new());
    let web_search_results = use_signal(|| Vec::<DagNode>::new());
    let contracts = use_signal(|| Vec::<DagNode>::new());
    let smart_contracts = use_signal(|| Vec::<DagNode>::new());
    let active_contract_history = use_signal(|| Vec::<DagNode>::new());
    let pending_contracts = use_signal(|| Vec::<DagNode>::new());
    let contract_states = use_signal(|| std::collections::HashMap::<String, String>::new());
    let proposals = use_signal(|| Vec::<DagNode>::new());
    let proposal_votes = use_signal(|| std::collections::HashMap::<String, Vec<DagNode>>::new());
    let proposal_tallies = use_signal(|| std::collections::HashMap::<String, (usize, usize, usize, usize, usize, String)>::new());
    let current_tax_rate = use_signal(|| 0u8);
    let candidates = use_signal(|| Vec::<DagNode>::new());
    let candidate_tallies = use_signal(|| std::collections::HashMap::<String, usize>::new());
    let recalls = use_signal(|| Vec::<DagNode>::new());
    let recall_tallies = use_signal(|| std::collections::HashMap::<String, (usize, usize, usize)>::new());
    let oversight_cases = use_signal(|| Vec::<DagNode>::new());
    let jury_duty = use_signal(|| Vec::<DagNode>::new());
    let reputation = use_signal(|| None::<backend::dag::ReputationDetails>);
    let my_web_pages = use_signal(|| Vec::<DagNode>::new());
    let reports = use_signal(|| Vec::<DagNode>::new());
    let files = use_signal(|| Vec::<DagNode>::new());
    let public_ledger = use_signal(|| Vec::<DagNode>::new());
    let file_search_results = use_signal(|| Vec::<DagNode>::new());
    let ministries = use_signal(|| Vec::<String>::new());
    let comments = use_signal(|| std::collections::HashMap::<String, Vec<DagNode>>::new());
    let likes = use_signal(|| std::collections::HashMap::<String, (usize, bool)>::new());
    let stories = use_signal(|| Vec::<DagNode>::new());
    let local_stories = use_signal(|| Vec::<DagNode>::new());
    let seen_stories = use_signal(|| HashSet::<String>::new());
    let following = use_signal(|| Vec::<String>::new());
    let user_posts = use_signal(|| Vec::<DagNode>::new());
    let following_posts = use_signal(|| Vec::<DagNode>::new());
    // Education System
    let courses = use_signal(|| Vec::<DagNode>::new());
    let exams = use_signal(|| Vec::<DagNode>::new());
    let certifications = use_signal(|| Vec::<DagNode>::new());
    let active_exam = use_signal(|| None::<DagNode>);
    let pending_applications = use_signal(|| Vec::<DagNode>::new());
    let exam_answers = use_signal(|| Vec::<Option<usize>>::new());
    let exam_result = use_signal(|| None::<(String, u8, bool)>);
    // Wiki homepage
    let all_web_pages = use_signal(|| Vec::<DagNode>::new());
        
    // Groups
    let groups = use_signal(|| Vec::<DagNode>::new());
    let group_messages = use_signal(|| std::collections::HashMap::<String, Vec<(DagNode, String)>>::new());
    
    use_context_provider(|| AppState { messages, blocks, history, user_profiles, page_title, browser_url, browser_content, active_tab, peers, local_peer_id, profile, balance, pending_transfers, geohash, ubi_timer, verification_status, viewed_profile, web_content, posts, blob_cache, last_created_blob, storage_stats, local_posts, listings, local_listings, web_search_results, contracts, smart_contracts, active_contract_history, pending_contracts, contract_states, proposals, proposal_votes, proposal_tallies, current_tax_rate, candidates, candidate_tallies, recalls,
        recall_tallies,
        oversight_cases,
        jury_duty,
        reputation, my_web_pages, reports, groups, group_messages, files, public_ledger, file_search_results, ministries, comments, likes, stories, local_stories, seen_stories, following, user_posts, following_posts, courses, exams, certifications, active_exam, pending_applications, exam_answers, exam_result, all_web_pages });

    // Initialize backend and context
    use_context_provider(|| {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<AppCmd>();
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();
        
        // Spawn the backend in a separate thread
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(backend::init(cmd_rx, event_tx, None));
        });

        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            backend::init(cmd_rx, event_tx, None).await;
        });

        // Spawn a task to handle events from the backend
        let cmd_tx_clone = cmd_tx.clone();
        spawn(async move {
            let mut sigs = EventSignals {
                messages,
                peers,
                local_peer_id,
                profile,
                balance,
                pending_transfers,
                geohash,
                ubi_timer,
                verification_status,
                viewed_profile,
                web_content,
                posts,
                blob_cache,
                last_created_blob,
                storage_stats,
                local_posts,
                listings,
                local_listings,
                web_search_results,
                contracts,
                active_contract_history,
                pending_contracts,
                contract_states,
                proposals,
                proposal_votes,
                proposal_tallies,
                current_tax_rate,
                candidates,
                candidate_tallies,
                recalls,
                recall_tallies,
                oversight_cases,
                jury_duty,
                reputation,
                my_web_pages,
                all_web_pages,
                groups,
                group_messages,
                reports,
                files,
                public_ledger,
                file_search_results,
                ministries,
                comments,
                likes,
                stories,
                local_stories,
                seen_stories,
                following,
                user_posts,
                following_posts,
                courses,
                exams,
                certifications,
                pending_applications,
                exam_result,
            };
            while let Some(event) = event_rx.recv().await {
                println!("UI Received event: {:?}", event);
                handle_app_event(event, &mut sigs, &cmd_tx_clone);
            }
        });

        // Initial checks
        let _ = cmd_tx.send(AppCmd::CheckVerificationStatus);
        let _ = cmd_tx.send(AppCmd::FetchStorageStats);
        let _ = cmd_tx.send(AppCmd::FetchMinistries);
        let _ = cmd_tx.send(AppCmd::FetchTaxRate);

        // Return the sender to be stored in context
        cmd_tx
    });

    rsx! {
        document::Stylesheet {href: asset!("assets/main.css")}
        Router::<Route> {}
    }
}

#[component]
fn RootLayout() -> Element {
    let app_state = use_context::<AppState>();
    let nav = use_navigator();
    let route = use_route::<Route>();

    use_effect(move || {
        let status = app_state.verification_status.read();
        let current_route = route.clone();
        
        if *status == backend::VerificationStatus::Unverified || *status == backend::VerificationStatus::EligibleForFounder {
            if current_route != (Route::VerificationPage {}) {
                nav.push(Route::VerificationPage {});
            }
        } else {
            // Verified or Founder
            if current_route == (Route::VerificationPage {}) {
                nav.push(Route::SuperWebShell {});
            }
        }
    });

    rsx! {
        Outlet::<Route> {}
    }
}