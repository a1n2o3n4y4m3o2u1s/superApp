use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::backend::dag::{DagPayload, PostPayload};
use base64::{Engine as _, engine::general_purpose};

#[component]
pub fn HomeComponent() -> Element {
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let app_state = use_context::<crate::components::AppState>();
    let posts = app_state.posts;
    
    let mut new_post_content = use_signal(|| "".to_string());
    let mut attached_cids = use_signal(|| Vec::<String>::new());
    
    let mut last_processed_blob = use_signal(|| None::<String>);
    use_effect(move || {
        if let Some(blob_id) = app_state.last_created_blob.read().clone() {
            if last_processed_blob() != Some(blob_id.clone()) {
                 attached_cids.write().push(blob_id.clone());
                 last_processed_blob.set(Some(blob_id));
            }
        }
    });

    let cmd_tx_clone = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_clone.send(AppCmd::FetchPosts);
    });

    let on_submit = {
        let cmd_tx = cmd_tx.clone();
        move |_| {
            if !new_post_content().is_empty() || !attached_cids().is_empty() {
                let cmd = AppCmd::PublishPost {
                    content: new_post_content(),
                    attachments: attached_cids().clone(),
                    geohash: None,
                };
                if let Err(e) = cmd_tx.send(cmd) {
                    eprintln!("Failed to send PublishPost command: {:?}", e);
                } else {
                    new_post_content.set("".to_string());
                    attached_cids.set(Vec::new());
                }
            }
        }
    };

    let upload_file = {
        let cmd_tx = cmd_tx.clone();
        move |evt: Event<FormData>| {
            let cmd_tx = cmd_tx.clone();
            let files: Vec<_> = evt.files().into_iter().collect();
            spawn(async move {
                for file_data in files {
                    let file_name = file_data.name();
                    
                    // Get MIME type from file or determine from extension
                    let mime_type = file_data.content_type().unwrap_or_else(|| {
                        if file_name.to_lowercase().ends_with(".png") {
                            "image/png".to_string()
                        } else if file_name.to_lowercase().ends_with(".gif") {
                            "image/gif".to_string()
                        } else if file_name.to_lowercase().ends_with(".webp") {
                            "image/webp".to_string()
                        } else {
                            "image/jpeg".to_string() // Default to JPEG for other images
                        }
                    });
                    
                    // Read file bytes
                    if let Ok(file_bytes) = file_data.read_bytes().await {
                        // Encode file data as base64
                        let data = general_purpose::STANDARD.encode(&file_bytes);
                        
                        // Send to backend
                        if let Err(e) = cmd_tx.send(AppCmd::PublishBlob { mime_type, data }) {
                            eprintln!("Failed to send PublishBlob command: {:?}", e);
                        }
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Compose area
            div { class: "panel mb-8",
                div { class: "panel-header",
                    h2 { class: "panel-title", "Share something" }
                }
                
                div { class: "form-group",
                    textarea {
                        class: "input",
                        style: "min-height: 100px; resize: none;",
                        placeholder: "What's on your mind?",
                        value: "{new_post_content}",
                        oninput: move |e| new_post_content.set(e.value())
                    }
                }
                
                if !attached_cids().is_empty() {
                    div { class: "flex gap-3 mb-4 overflow-x-auto pb-2",
                        for cid in attached_cids() {
                            div { class: "relative w-20 h-20 rounded-lg overflow-hidden border border-[var(--border-default)] flex-shrink-0",
                                BlobImage { cid: cid.clone() }
                                button {
                                    class: "absolute top-1 right-1 w-5 h-5 bg-black/70 hover:bg-black rounded-full flex items-center justify-center text-xs",
                                    onclick: move |_| {
                                         attached_cids.write().retain(|c| c != &cid);
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }

                div { class: "flex justify-between items-center pt-4 border-t border-[var(--border-subtle)]",
                    div { class: "relative",
                        button { class: "btn btn-secondary btn-sm",
                            "📷 Add Image"
                        }
                        input {
                            class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                            r#type: "file",
                            accept: "image/*",
                            onchange: upload_file
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: on_submit,
                        "Post"
                    }
                }
            }

            // Feed
            div { class: "panel",
                if posts().is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", "📝" }
                        p { class: "empty-state-title", "No posts yet" }
                        p { class: "empty-state-text", "Be the first to share something!" }
                    }
                } else {
                    for node in posts() {
                        if let DagPayload::Post(PostPayload { content, attachments, .. }) = &node.payload {
                            div { 
                                class: "post",
                                key: "{node.id}",
                                
                                div { class: "post-header",
                                    div { class: "avatar",
                                        "{node.author.get(0..2).unwrap_or(\"??\")}"
                                    }
                                    div { class: "flex-1",
                                        Link {
                                            to: crate::Route::UserProfileComponent { peer_id: node.author.clone() },
                                            class: "post-author",
                                            "{node.author.get(0..12).unwrap_or(&node.author)}..."
                                        }
                                    }
                                    span { class: "post-time", "{node.timestamp}" }
                                }
                                
                                p { class: "post-content", "{content}" }
                                
                                if !attachments.is_empty() {
                                    div { class: "post-attachments",
                                        for cid in attachments {
                                            div { class: "post-attachment",
                                                BlobImage { cid: cid.clone() }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn BlobImage(cid: String) -> Element {
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

    rsx! {
        div { class: "w-full h-full",
            if let Some(s) = src {
                img {
                    src: "{s}",
                    class: "w-full h-auto object-cover"
                }
            } else {
                div { class: "h-24 w-full bg-[var(--bg-surface)] animate-pulse flex items-center justify-center",
                     span { class: "text-xs text-[var(--text-muted)]", "Loading..." }
                }
            }
        }
    }
}