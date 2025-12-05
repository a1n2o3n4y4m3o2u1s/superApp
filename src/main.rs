mod backend;
mod components;

use components::home_page;
use components::messaging_page;
use components::nav_bar;

use home_page::HomeComponent;
use messaging_page::MessagingComponent;
use nav_bar::NavComponent;
use components::profile_page;
use profile_page::{ProfileComponent, UserProfileComponent};
use components::geohash_page;
use geohash_page::GeohashComponent;
use components::verification_page;
use verification_page::VerificationPage;
use components::browser_page;
use browser_page::BrowserComponent;
use components::marketplace_page;
use marketplace_page::MarketplaceComponent;

use dioxus::prelude::*;
use tokio::sync::mpsc;
use backend::{AppCmd, AppEvent};

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(RootLayout)]
        #[layout(NavComponent)]
        #[route("/")]
        HomeComponent {},
        #[route("/messaging")]
        MessagingComponent {},
        #[route("/profile")]
        ProfileComponent {},
        #[route("/geohash")]
        GeohashComponent {},
        #[route("/profile/:peer_id")]
        UserProfileComponent { peer_id: String },
        #[route("/browser")]
        BrowserComponent {},
        #[route("/marketplace")]
        MarketplaceComponent {},
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
    let mut contract_states = use_signal(|| std::collections::HashMap::<String, String>::new());
    
    use_context_provider(|| AppState { messages, peers, local_peer_id, profile, balance, pending_transfers, geohash, ubi_timer, verification_status, viewed_profile, web_content, posts, blob_cache, last_created_blob, storage_stats, local_posts, listings, web_search_results, contracts, contract_states });

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
                        local_peer_id.set(id);
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
                    _ => {}
                }
            }
        });

        // Initial checks
        let _ = cmd_tx.send(AppCmd::CheckVerificationStatus);
        let _ = cmd_tx.send(AppCmd::FetchStorageStats);

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
                nav.push(Route::HomeComponent {});
            }
        }
    });

    rsx! {
        Outlet::<Route> {}
    }
}