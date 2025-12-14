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

        #[route("/smart_contracts")]
        SmartContractsPage {},
        #[end_layout]
    
    #[route("/verify")]
    VerificationPage {},
    
}

use std::collections::HashSet;
use backend::dag::DagNode;
use components::AppState;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Initialize global state
    let mut messages = use_signal(|| Vec::<(DagNode, String)>::new());
    let blocks = use_signal(|| Vec::<DagNode>::new());
    let history = use_signal(|| Vec::<DagNode>::new());
    let user_profiles = use_signal(|| std::collections::HashMap::<String, backend::dag::ProfilePayload>::new());
    let page_title = use_signal(|| "SuperApp".to_string());
    let browser_url = use_signal(|| "sp://welcome".to_string());
    let browser_content = use_signal(|| None::<String>);
    let active_tab = use_signal(|| "feed".to_string());
    let mut peers = use_signal(|| HashSet::<String>::new());
    let mut local_peer_id = use_signal(|| String::new());
    let mut profile = use_signal(|| None::<backend::dag::ProfilePayload>);
    let mut balance = use_signal(|| 0i64);
    let mut pending_transfers = use_signal(|| Vec::<DagNode>::new());
    let mut geohash = use_signal(|| "Global".to_string());
    let mut ubi_timer = use_signal(|| None::<u64>);
    let mut verification_status = use_signal(|| backend::VerificationStatus::Unverified);
    let mut viewed_profile = use_signal(|| None::<backend::dag::ProfilePayload>);
    let mut web_content = use_signal(|| None::<String>);
    let mut posts = use_signal(|| Vec::<DagNode>::new());
    let mut blob_cache = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut last_created_blob = use_signal(|| None::<String>);
    let mut storage_stats = use_signal(|| (0usize, 0usize));
    let mut local_posts = use_signal(|| Vec::<DagNode>::new());
    let mut listings = use_signal(|| Vec::<DagNode>::new());
    let mut web_search_results = use_signal(|| Vec::<DagNode>::new());
    let mut contracts = use_signal(|| Vec::<DagNode>::new());
    let mut smart_contracts = use_signal(|| Vec::<DagNode>::new());
    let mut active_contract_history = use_signal(|| Vec::<DagNode>::new());
    let mut pending_contracts = use_signal(|| Vec::<DagNode>::new());
    let mut contract_states = use_signal(|| std::collections::HashMap::<String, String>::new());
    let mut proposals = use_signal(|| Vec::<DagNode>::new());
    let mut proposal_votes = use_signal(|| std::collections::HashMap::<String, Vec<DagNode>>::new());
    let mut proposal_tallies = use_signal(|| std::collections::HashMap::<String, (usize, usize, usize, usize, usize, String)>::new());
    let mut current_tax_rate = use_signal(|| 0u8);
    let mut candidates = use_signal(|| Vec::<DagNode>::new());
    let mut candidate_tallies = use_signal(|| std::collections::HashMap::<String, usize>::new());
    let mut recalls = use_signal(|| Vec::<DagNode>::new());
    let mut recall_tallies = use_signal(|| std::collections::HashMap::<String, (usize, usize, usize)>::new());
    let mut oversight_cases = use_signal(|| Vec::<DagNode>::new());
    let mut jury_duty = use_signal(|| Vec::<DagNode>::new());
    let mut reputation = use_signal(|| None::<backend::dag::ReputationDetails>);
    let mut my_web_pages = use_signal(|| Vec::<DagNode>::new());
    let mut reports = use_signal(|| Vec::<DagNode>::new());
    let mut files = use_signal(|| Vec::<DagNode>::new());
    let mut public_ledger = use_signal(|| Vec::<DagNode>::new());
    let mut file_search_results = use_signal(|| Vec::<DagNode>::new());
    let mut ministries = use_signal(|| Vec::<String>::new());
    let mut comments = use_signal(|| std::collections::HashMap::<String, Vec<DagNode>>::new());
    let mut likes = use_signal(|| std::collections::HashMap::<String, (usize, bool)>::new());
    let mut stories = use_signal(|| Vec::<DagNode>::new());
    let seen_stories = use_signal(|| HashSet::<String>::new());
    let mut following = use_signal(|| HashSet::<String>::new());
    let mut user_posts = use_signal(|| Vec::<DagNode>::new());
    let mut following_posts = use_signal(|| Vec::<DagNode>::new());
    // Education System
    let mut courses = use_signal(|| Vec::<DagNode>::new());
    let mut exams = use_signal(|| Vec::<DagNode>::new());
    let mut certifications = use_signal(|| Vec::<DagNode>::new());
    let active_exam = use_signal(|| None::<DagNode>);
    let mut pending_applications = use_signal(|| Vec::<DagNode>::new());
    let exam_answers = use_signal(|| Vec::<Option<usize>>::new());
    let mut exam_result = use_signal(|| None::<(String, u8, bool)>);
    // Wiki homepage
    let mut all_web_pages = use_signal(|| Vec::<DagNode>::new());
        
    // Groups
    let mut groups = use_signal(|| Vec::<DagNode>::new());
    let mut group_messages = use_signal(|| std::collections::HashMap::<String, Vec<(DagNode, String)>>::new());
    
    use_context_provider(|| AppState { messages, blocks, history, user_profiles, page_title, browser_url, browser_content, active_tab, peers, local_peer_id, profile, balance, pending_transfers, geohash, ubi_timer, verification_status, viewed_profile, web_content, posts, blob_cache, last_created_blob, storage_stats, local_posts, listings, web_search_results, contracts, smart_contracts, active_contract_history, pending_contracts, contract_states, proposals, proposal_votes, proposal_tallies, current_tax_rate, candidates, candidate_tallies, recalls,
        recall_tallies,
        oversight_cases,
        jury_duty,
        reputation, my_web_pages, reports, groups, group_messages, files, public_ledger, file_search_results, ministries, comments, likes, stories, seen_stories, following, user_posts, following_posts, courses, exams, certifications, active_exam, pending_applications, exam_answers, exam_result, all_web_pages });

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
            while let Some(event) = event_rx.recv().await {
                println!("UI Received event: {:?}", event);
                match event {
                    AppEvent::MessageReceived(node, content) => {
                        messages.write().push((node, content));
                    }
                    AppEvent::MessagesFetched(msgs) => {
                        *messages.write() = msgs;
                    }
                    AppEvent::PeerDiscovered(peer) => {
                        peers.write().insert(peer);
                    }
                    AppEvent::MyIdentity(id) => {
                        local_peer_id.set(id.clone());
                        let _ = cmd_tx_clone.send(AppCmd::FetchFollowing { target: id });
                    }
                    AppEvent::ProfileFetched(p) => {
                        profile.set(p);
                    }
                    AppEvent::BalanceFetched(bal) => {
                        balance.set(bal);
                    }
                    AppEvent::PendingTransfersFetched(nodes) => {
                        pending_transfers.set(nodes);
                    }
                    AppEvent::GeohashDetected(hash) => {
                        geohash.set(hash);
                    }
                    AppEvent::UbiTimerFetched(time) => {
                        ubi_timer.set(time);
                    }
                    AppEvent::VerificationStatus(status) => {
                        println!("Verification status update: {:?}", status);
                        verification_status.set(status.clone());
                    }
                    AppEvent::UserProfileFetched(p) => {
                        viewed_profile.set(p);
                    }
                    AppEvent::WebPageFetched { url, content } => {
                        if let Some(c) = content {
                            web_content.set(Some(c));
                        } else {
                            web_content.set(Some(format!("<h1>404 Not Found</h1><p>Could not find page: {}</p>", url)));
                        }
                    }
                    AppEvent::HistoryFetched(fetched_posts) => {
                        posts.set(fetched_posts);
                    }
                    AppEvent::BlockReceived(node) => {
                         match node.r#type.as_str() {
                             "post:v1" => {
                                 posts.write().insert(0, node.clone());
                             }
                             "blob:v1" => {
                                 if let backend::dag::DagPayload::Blob(blob) = &node.payload {
                                     blob_cache.write().insert(node.id.clone(), format!("data:{};base64,{}", blob.mime_type, blob.data));
                                     if node.author == local_peer_id.read().clone() {
                                         last_created_blob.set(Some(node.id.clone()));
                                     }
                                 }
                             }
                             "proposal:v1" => {
                                 proposals.write().insert(0, node.clone());
                             }
                             "vote:v1" => {
                                 if let backend::dag::DagPayload::Vote(v) = &node.payload {
                                     let mut votes_map = proposal_votes.write();
                                     let entry = votes_map.entry(v.proposal_id.clone()).or_insert(Vec::new());
                                     entry.push(node.clone());
                                 }
                             }
                             "candidacy:v1" => {
                                 candidates.write().insert(0, node.clone());
                             }
                             "candidacy_vote:v1" => {
                                 // Tally will be fetched separately, just trigger a refresh
                             }
                             "file:v1" => {
                                 files.write().insert(0, node.clone());
                             }
                             "story:v1" => {
                                 stories.write().insert(0, node.clone());
                             }
                             "follow:v1" => {
                                 if node.author == local_peer_id.read().clone() {
                                     if let backend::dag::DagPayload::Follow(f) = &node.payload {
                                         if f.follow {
                                             following.write().insert(f.target.clone());
                                         } else {
                                             following.write().remove(&f.target);
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
                                     blob_cache.write().insert(n.id.clone(), format!("data:{};base64,{}", blob.mime_type, blob.data));
                                 }
                            }
                        }
                    }
                    AppEvent::StorageStatsFetched { block_count, total_bytes } => {
                        storage_stats.set((block_count, total_bytes));
                    }
                    AppEvent::LocalPostsFetched(fetched_posts) => {
                        local_posts.set(fetched_posts);
                    }
                    AppEvent::ListingsFetched(fetched_listings) => {
                        listings.set(fetched_listings);
                    }
                    AppEvent::WebSearchResults(results) => {
                        web_search_results.set(results);
                    }
                    AppEvent::ContractsFetched(fetched_contracts) => {
                        contracts.set(fetched_contracts);
                    }

                    AppEvent::ContractStateFetched { contract_id, state } => {
                        contract_states.write().insert(contract_id, state);
                    }
                    AppEvent::ContractHistoryFetched { contract_id: _, history } => {
                         active_contract_history.set(history);
                    }
                    AppEvent::PendingContractsFetched(contracts) => {
                         pending_contracts.set(contracts);
                    }
                    AppEvent::ProposalsFetched(fetched_proposals) => {
                        proposals.set(fetched_proposals);
                    }
                    AppEvent::ProposalVotesFetched { proposal_id, votes } => {
                        proposal_votes.write().insert(proposal_id, votes);
                    }
                    AppEvent::ProposalTallyFetched { proposal_id, yes, no, abstain, petition, unique_voters, status } => {
                        proposal_tallies.write().insert(proposal_id, (yes, no, abstain, petition, unique_voters, status));
                    }
                    AppEvent::TaxRateFetched(rate) => {
                        current_tax_rate.set(rate);
                    }
                    AppEvent::CandidatesFetched(fetched_candidates) => {
                        candidates.set(fetched_candidates);
                    }
                    AppEvent::CandidateTallyFetched { candidacy_id, votes } => {
                        candidate_tallies.write().insert(candidacy_id, votes);
                    }
                    AppEvent::RecallsFetched(fetched_recalls) => {
                        recalls.set(fetched_recalls);
                    }
                    AppEvent::RecallTallyFetched { recall_id, remove, keep, unique_voters } => {
                        recall_tallies.write().insert(recall_id, (remove, keep, unique_voters));
                    }
                    AppEvent::ReputationFetched(details) => {
                    reputation.set(Some(details));
                }
                AppEvent::OversightCasesFetched(cases) => {
                    oversight_cases.set(cases);
                }
                AppEvent::JuryDutyFetched(cases) => {
                    jury_duty.set(cases);
                }    
                    AppEvent::MyWebPagesFetched(pages) => {
                        my_web_pages.set(pages);
                    }
                    AppEvent::AllWebPagesFetched(pages) => {
                        all_web_pages.set(pages);
                    }
                    AppEvent::GroupsFetched(fetched_groups) => {
                        groups.set(fetched_groups);
                    }
                    AppEvent::GroupMessagesFetched(msgs) => {
                         if !msgs.is_empty() {
                             if let backend::dag::DagPayload::Message(p) = &msgs[0].0.payload {
                                 if let Some(gid) = &p.group_id {
                                     group_messages.write().insert(gid.clone(), msgs);
                                 }
                             }
                         }
                    }
                    AppEvent::ReportsFetched(fetched_reports) => {
                        reports.set(fetched_reports);
                    }
                    AppEvent::MyFilesFetched(fetched_files) => {
                        files.set(fetched_files);
                    }
                    AppEvent::FileUploaded(node) => {
                        // Already handled via BlockReceived usually, but ensure it's in list if not
                    }
                    AppEvent::PublicLedgerFetched(events) => {
                        public_ledger.set(events);
                    }
                    AppEvent::FileSearchResults(results) => {
                        file_search_results.set(results);
                    }
                    AppEvent::MinistriesFetched(m) => {
                        ministries.set(m);
                    }
                    AppEvent::CommentsFetched { parent_id, comments: c } => {
                        comments.write().insert(parent_id, c);
                    }
                    AppEvent::LikesFetched { target_id, count, is_liked_by_me } => {
                        likes.write().insert(target_id, (count, is_liked_by_me));
                    }
                    AppEvent::StoriesFetched(s) => {
                        stories.set(s);
                    }
                    AppEvent::FollowingFetched(f) => {
                        let mut set = HashSet::new();
                        for id in f {
                            set.insert(id);
                        }
                        following.set(set);
                    }
                    AppEvent::UserPostsFetched(p) => {
                        user_posts.set(p);
                    }
                    AppEvent::FollowingPostsFetched(p) => {
                        following_posts.set(p);
                    }
                    // Education System events
                    AppEvent::CoursesFetched(c) => {
                        courses.set(c);
                    }
                    AppEvent::ExamsFetched(e) => {
                        exams.set(e);
                    }
                    AppEvent::CertificationsFetched(c) => {
                        certifications.set(c);
                    }
                    AppEvent::ExamSubmitted { exam_id, score, passed } => {
                        println!("Exam {} submitted: score={}, passed={}", exam_id, score, passed);
                        exam_result.set(Some((exam_id, score, passed)));
                    }
                    AppEvent::PendingApplicationsFetched(apps) => {
                        pending_applications.set(apps);
                    }
                    AppEvent::ApplicationVotesFetched { .. } => {
                        // Handled locally in UI
                    }
                    _ => {}
                }
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
        document::Stylesheet {href: asset!("/assets/main.css")}
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