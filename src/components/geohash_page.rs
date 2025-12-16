use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::{DagPayload, PostPayload}};

#[component]
pub fn GeohashComponent() -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();

    let mut precision = use_signal(|| 3usize); // Default: country level
    let mut new_local_post = use_signal(|| String::new());
    
    let current_geohash = app_state.geohash.read().clone();
    let geohash_prefix = if current_geohash == "Global" {
        String::new()
    } else {
        current_geohash.chars().take(precision()).collect::<String>()
    };
    let prefix_display = if geohash_prefix.is_empty() { "Global".to_string() } else { geohash_prefix.clone() };

    // Auto-detect geohash on mount
    let cmd_tx_effect = cmd_tx.clone();
    let geohash_announce = current_geohash.clone();
    let cmd_tx_announce = cmd_tx.clone();
    
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::AutoDetectGeohash);
    });
    
    // Announce presence when geohash is resolved
    use_effect(move || {
         if !geohash_announce.is_empty() && geohash_announce != "Global" {
             let _ = cmd_tx_announce.send(AppCmd::AnnouncePresence { geohash: geohash_announce.clone() });
         }
    });

    // Fetch local posts when precision or geohash changes
    let cmd_tx_fetch = cmd_tx.clone();
    let geohash_prefix_fetch = geohash_prefix.clone();
    use_effect(move || {
        if !geohash_prefix_fetch.is_empty() {
            let _ = cmd_tx_fetch.send(AppCmd::FetchLocalPosts { geohash_prefix: geohash_prefix_fetch.clone() });
        }
    });

    let on_post_local = {
        let cmd_tx = cmd_tx.clone();
        let geohash_prefix = geohash_prefix.clone();
        move |_| {
            if !new_local_post().is_empty() && !geohash_prefix.is_empty() {
                let cmd = AppCmd::PublishPost {
                    content: new_local_post(),
                    attachments: vec![],
                    geohash: Some(geohash_prefix.clone()),
                    announcement: false,
                };
                let _ = cmd_tx.send(cmd);
                new_local_post.set(String::new());
                
                // Refresh local posts
                let _ = cmd_tx.send(AppCmd::FetchLocalPosts { geohash_prefix: geohash_prefix.clone() });
            }
        }
    };

    let cmd_tx_refresh = cmd_tx.clone();
    let geohash_prefix_refresh = geohash_prefix.clone();
    let on_refresh = move |_| {
        if !geohash_prefix_refresh.is_empty() {
            let _ = cmd_tx_refresh.send(AppCmd::FetchLocalPosts { geohash_prefix: geohash_prefix_refresh.clone() });
        }
    };

    let precision_labels = ["Global", "Continent", "Country", "Region", "City", "Neighborhood"];

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Header
            div { class: "page-header",
                div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "page-title", "Local Area" }
                        p { class: "text-[var(--text-secondary)] mt-1",
                            "Location: "
                            span { class: "font-mono", "{geohash_prefix}" }
                            if geohash_prefix.is_empty() {
                                span { " (detecting...)" }
                            }
                        }
                    }
                }
            }

            // Precision selector
            div { class: "panel mb-6",
                div { class: "panel-header",
                    h2 { class: "panel-title", "Precision" }
                }
                div { class: "flex flex-wrap gap-2",
                    for (i, label) in precision_labels.iter().enumerate() {
                        button {
                            class: if precision() == i { "btn btn-primary btn-sm" } else { "btn btn-secondary btn-sm" },
                            onclick: move |_| precision.set(i),
                            "{label}"
                        }
                    }
                }
            }

            // Content grid
            div { class: "grid grid-cols-1 lg:grid-cols-3 gap-6",
                
                // Main content
                div { class: "lg:col-span-2 section-stack",
                    
                    // Post locally
                    if !geohash_prefix.is_empty() {
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "Post Here" }
                            }
                            div { class: "form-group",
                                textarea {
                                    class: "input",
                                    style: "min-height: 80px; resize: none;",
                                    placeholder: "Share something with your local area...",
                                    value: "{new_local_post}",
                                    oninput: move |e| new_local_post.set(e.value())
                                }
                            }
                            button { class: "btn btn-primary", onclick: on_post_local, "Post Locally" }
                        }
                    }
                    
                    // Local feed
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "Local Feed" }
                            div { class: "flex gap-2",
                                button { class: "btn btn-primary btn-sm", onclick: on_refresh, "Refresh" }
                            }
                        }
                        if app_state.local_posts.read().is_empty() {
                            div { class: "empty-state",
                                div { class: "empty-state-icon", "📍" }
                                p { class: "empty-state-title", "No local posts yet" }
                                p { class: "empty-state-text", "Be the first to post in this area!" }
                            }
                        } else {
                            div { class: "space-y-4",
                                for node in app_state.local_posts.read().iter() {
                                    if let DagPayload::Post(PostPayload { content, geohash: post_gh, .. }) = &node.payload {
                                        div {
                                            class: "post",
                                            key: "{node.id}",
                                            
                                            div { class: "post-header",
                                                div { class: "avatar",
                                                    "{node.author.get(0..2).unwrap_or(\"??\")}"
                                                }
                                                div { class: "flex-1",
                                                    p { class: "post-author",
                                                        "{node.author.get(0..12).unwrap_or(&node.author)}..."
                                                    }
                                                    if let Some(gh) = post_gh {
                                                        span { class: "text-xs text-[var(--text-muted)] ml-2", "📍 {gh}" }
                                                    }
                                                }
                                                span { class: "post-time", "{node.timestamp}" }
                                            }
                                            
                                            p { class: "post-content", "{content}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Nearby users
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "Nearby Users" }
                        }
                        if app_state.peers.read().is_empty() {
                            div { class: "empty-state py-8",
                                p { class: "empty-state-text", "No users found nearby" }
                            }
                        } else {
                            div { class: "space-y-2",
                                {app_state.peers.read().iter().map(|peer_id| {
                                    let pid = peer_id.clone();
                                    let cmd_tx_vouch = cmd_tx.clone();
                                    rsx! {
                                        div {
                                            class: "list-item",
                                            key: "{pid}",
                                            div { class: "avatar avatar-sm",
                                                "{pid.get(0..2).unwrap_or(\"??\")}"
                                            }
                                            div { class: "list-item-content",
                                                p { class: "list-item-title font-mono text-sm truncate", "{pid}" }
                                            }
                                            button {
                                                class: "btn btn-secondary btn-sm",
                                                onclick: move |_| {
                                                    let _ = cmd_tx_vouch.send(AppCmd::Vouch { target_peer_id: pid.clone() });
                                                },
                                                "Vouch"
                                            }
                                        }
                                    }
                                })}
                            }
                        }
                    }
                }

                // Sidebar
                div { class: "panel h-fit",
                    div { class: "panel-header",
                        h2 { class: "panel-title", "Area Info" }
                    }
                    div { class: "card",
                        p { class: "label", "Current Geohash" }
                        p { class: "font-mono text-lg mt-1", "{app_state.geohash}" }
                    }
                    div { class: "card mt-4",
                        p { class: "label", "Viewing Prefix" }
                        p { class: "font-mono text-lg mt-1", "{prefix_display}" }
                    }
                    div { class: "card mt-4",
                        p { class: "label", "Precision Level" }
                        p { class: "text-lg mt-1", "{precision_labels[precision()]}" }
                    }
                }
            }
        }
    }
}
