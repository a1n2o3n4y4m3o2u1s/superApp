use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::backend::dag::{DagNode, DagPayload};

#[component]
pub fn BlobImage(cid: String, class: Option<String>) -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let cache = app_state.blob_cache.read();
    let src = cache.get(&cid).cloned();
    drop(cache);
    
    let cid_clone = cid.clone();
    use_effect(move || {
        let app_state = app_state.clone();
        let cmd_tx = cmd_tx.clone();
        if !app_state.blob_cache.read().contains_key(&cid_clone) {
             let _ = cmd_tx.send(AppCmd::FetchBlock { cid: cid_clone.clone(), peer_id: None });
        }
    });

    let extra_class = class.unwrap_or_default();

    rsx! {
        div { class: "w-full h-full {extra_class}",
            if let Some(s) = src {
                img {
                    src: "{s}",
                    class: "w-full h-full object-cover rounded-inherit"
                }
            } else {
                div { class: "h-full w-full bg-[var(--bg-surface)] animate-pulse flex items-center justify-center rounded-inherit",
                     span { class: "text-[10px] text-[var(--text-muted)]", "..." }
                }
            }
        }
    }
}

#[component]
pub fn StoryCircle(
    node: DagNode,
    is_seen: bool,
    onclick: EventHandler<DagNode>,
) -> Element {
    if let DagPayload::Story(story) = &node.payload {
        let ring_class = if is_seen {
            "bg-gray-700"
        } else {
            "bg-gradient-to-tr from-yellow-400 to-fuchsia-600"
        };
        
        let cid = story.media_cid.clone();
        let author_short = node.author.get(0..4).unwrap_or("??").to_string();

        rsx! {
            div { 
                class: "inline-block cursor-pointer flex-shrink-0 animate-scale-in",
                onclick: move |_| onclick.call(node.clone()),
                div { class: "w-16 h-16 rounded-full p-[2px] {ring_class}",
                    div { class: "w-full h-full rounded-full border-2 border-[var(--bg-default)] overflow-hidden bg-[var(--bg-secondary)]",
                        BlobImage { cid: cid }
                    }
                }
                div { class: "text-[10px] text-center mt-1 truncate w-16 text-[var(--text-secondary)]", "{author_short}" }
            }
        }
    } else {
        rsx! { div { "Invalid Node" } }
    }
}

#[component]
pub fn StoryViewer(
    node: DagNode,
    on_close: EventHandler<()>,
) -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let user_profiles = app_state.user_profiles.read();
    
    if let DagPayload::Story(story) = &node.payload {
        let author_name = if let Some(profile) = user_profiles.get(&node.author) {
            profile.name.clone()
        } else {
            format!("{}...", &node.author[0..8])
        };
        
        let profile_photo = user_profiles.get(&node.author).and_then(|p| p.photo.clone());
        let time_str = node.timestamp.format("%H:%M").to_string();
        let cid = story.media_cid.clone();
        let caption = story.caption.clone();

        rsx! {
            div { 
                class: "fixed inset-0 z-[100] bg-black/95 backdrop-blur-md flex items-center justify-center animate-fade-in",
                onclick: move |_| on_close.call(()),
                
                div { 
                    class: "relative max-w-lg w-full h-full max-h-[90vh] flex flex-col items-center justify-center",
                    onclick: move |e| e.stop_propagation(),
                    
                    // Header
                    div { class: "absolute top-4 left-4 right-4 z-10 flex items-center justify-between",
                        div { class: "flex items-center gap-2",
                            div { class: "w-10 h-10 rounded-full bg-[var(--bg-secondary)] border border-white/20 overflow-hidden flex items-center justify-center",
                                if let Some(photo) = profile_photo {
                                    BlobImage { cid: photo }
                                } else {
                                    span { class: "text-white text-xs", "{node.author.get(0..2).unwrap_or(\"??\")}" }
                                }
                            }
                            div {
                                div { class: "text-white font-bold text-sm drop-shadow-lg", "{author_name}" }
                                div { class: "text-white/70 text-[10px] drop-shadow-lg", "{time_str}" }
                            }
                        }
                        
                        button { 
                            class: "w-10 h-10 rounded-full bg-white/10 hover:bg-white/20 flex items-center justify-center text-white transition-colors",
                            onclick: move |_| on_close.call(()),
                            "✕"
                        }
                    }
                    
                    // Main Media
                    div { class: "w-full flex-1 flex items-center justify-center p-2",
                        div { class: "max-w-full max-h-full rounded-xl overflow-hidden shadow-2xl shadow-white/5",
                            BlobImage { cid: cid }
                        }
                    }
                    
                    // Caption
                    if !caption.is_empty() {
                        div { class: "w-full p-6 text-center text-white bg-gradient-to-t from-black/80 to-transparent",
                            p { class: "text-sm md:text-base font-medium", "{caption}" }
                        }
                    }
                }
            }
        }
    } else {
        rsx! { div {} }
    }
}
