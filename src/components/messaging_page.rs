use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::DagPayload};
use crate::components::AppState;
use tokio::sync::mpsc::UnboundedSender;
use aes_gcm::{Aes256Gcm, KeyInit, aead::Aead, aead::AeadCore};

use rand::rngs::OsRng;
use base64::{Engine as _, engine::general_purpose};

#[component]
pub fn MessagingComponent() -> Element {
    let app_state = use_context::<AppState>();
    let mut input_msg = use_signal(|| String::new());
    
    // Selection state
    let mut active_group = use_signal(|| Option::<String>::None); // Group ID
    let mut target_peer = use_signal(|| String::new()); // Peer ID for DM

    // Modals
    let mut show_new_chat_modal = use_signal(|| false);
    let mut show_create_group_modal = use_signal(|| false);
    
    // New Group Form
    let mut new_group_name = use_signal(|| String::new());
    let mut selected_peers_for_group = use_signal(|| std::collections::HashSet::<String>::new());

    // Search
    let mut search_query = use_signal(|| String::new());
    
    // File upload state
    let mut is_uploading = use_signal(|| false);
    let mut last_uploaded_file_info = use_signal(|| None::<(String, String, String, String, String)>); // (cid, key, nonce, mime, filename)
    let mut pending_upload = use_signal(|| None::<(String, String, String, String)>); // (key, nonce, mime, filename)

    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    
    let target = target_peer.read().clone();
    let current_group = active_group.read().clone();
    let local_id = app_state.local_peer_id.read().clone();

    // Effects for fetching messages
    let cmd_tx_effect = cmd_tx.clone();
    let cmd_tx_effect2 = cmd_tx.clone();
    let target_effect = target.clone();
    let group_effect = current_group.clone();
    let mut viewed_profile = app_state.viewed_profile;
    
    use_effect(move || {
        let t = target_effect.clone();
        if !t.is_empty() {
             *viewed_profile.write() = None; // Clear stale profile
             let _ = cmd_tx_effect.send(AppCmd::FetchMessages { peer_id: t.clone() });
             let _ = cmd_tx_effect.send(AppCmd::FetchUserProfile { peer_id: t });
        }
    });

    use_effect(move || {
        let g = group_effect.clone();
        if let Some(gid) = g {
            let _ = cmd_tx_effect2.send(AppCmd::FetchGroupMessages { group_id: gid });
        } else {
             let _ = cmd_tx_effect2.send(AppCmd::FetchGroups);
        }
    });

    // Handle Blob Created Event
    use_effect(move || {
        let blob_id_opt = app_state.last_created_blob.read().clone();
        if let Some(blob_id) = blob_id_opt {
             let pending_opt = pending_upload.read().clone();
             if let Some((key, nonce, mime, filename)) = pending_opt {
                let info = (blob_id.clone(), key, nonce, mime, filename);
                last_uploaded_file_info.set(Some(info));
                pending_upload.set(None);
                is_uploading.set(false);
            }
        }
    });

    // File Upload Logic
    let upload_file = {
        let cmd_tx = cmd_tx.clone();
        move |evt: Event<FormData>| {
            let cmd_tx = cmd_tx.clone();
            let files: Vec<_> = evt.files().into_iter().collect();
            if files.is_empty() { return; }
            
            is_uploading.set(true);
            
            spawn(async move {
                if let Some(file_data) = files.first() {
                    let filename = file_data.name();
                    let mime_type = file_data.content_type().unwrap_or("application/octet-stream".to_string());
                    
                    if let Ok(file_bytes) = file_data.read_bytes().await {
                         let key = Aes256Gcm::generate_key(&mut OsRng);
                         let cipher = Aes256Gcm::new(&key);
                         let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                         
                         if let Ok(ciphertext) = cipher.encrypt(&nonce, file_bytes.as_ref()) {
                             let data_base64 = general_purpose::STANDARD.encode(&ciphertext);
                             let _ = cmd_tx.send(AppCmd::PublishBlob { mime_type: "application/encrypted".to_string(), data: data_base64 });
                             
                             let key_hex = hex::encode(key);
                             let nonce_hex = hex::encode(nonce);
                             pending_upload.set(Some((key_hex, nonce_hex, mime_type, filename)));
                         } else {
                             eprintln!("Encryption failed");
                             is_uploading.set(false);
                         }
                    } else {
                        is_uploading.set(false);
                    }
                }
            });
        }
    };

    // Append uploaded file to input
    use_effect(move || {
        let last_upload_opt = last_uploaded_file_info.read().clone();
        if let Some((cid, key, nonce, mime, filename)) = last_upload_opt {
             let current = input_msg.read().clone();
             let new_content = if current.is_empty() {
                 format!("[FILE:{}:{}:{}:{}:{}]", cid, key, nonce, mime, filename)
             } else {
                 format!("{} [FILE:{}:{}:{}:{}:{}]", current, cid, key, nonce, mime, filename)
             };
             input_msg.set(new_content);
             last_uploaded_file_info.set(None);
        }
    });

    // Prepare message list
    let messages_list = if let Some(gid) = &current_group {
        app_state.group_messages.read().get(gid).cloned().unwrap_or_default().iter().map(|(msg, content)| {
             let is_from_me = msg.author == local_id;
             rsx! {
                MessageItem { 
                    msg: msg.clone(), 
                    is_me: is_from_me,
                    content: content.clone()
                }
            }
        }).collect::<Vec<_>>()
    } else {
        app_state.messages.read().iter().filter_map(|(msg, content)| {
            if let DagPayload::Message(payload) = &msg.payload {
                if payload.group_id.is_some() { return None; }
                
                let is_from_me = msg.author == local_id;
                let is_from_target = msg.author == target;
                let is_to_target = payload.recipient == target;
                let is_to_me = payload.recipient == local_id;

                if (is_from_me && is_to_target) || (is_from_target && is_to_me) {
                    return Some(rsx! {
                        MessageItem { 
                            msg: msg.clone(), 
                            is_me: is_from_me,
                            content: content.clone()
                        }
                    });
                }
            }
            None
        }).collect::<Vec<_>>()
    };

    // Combined Chat List Logic
    let groups = app_state.groups.read();
    let peers = app_state.peers.read();
    
    // We want to combine these into a list of "Chats"
    // For MVP, we iterate both and filter by search query
    
    let query = search_query.read().to_lowercase();
    
    let group_list_items = groups.iter().filter_map(|group_node| {
        if let DagPayload::Group(g) = &group_node.payload {
            if !query.is_empty() && !g.name.to_lowercase().contains(&query) {
                return None;
            }
            let gid = group_node.id.clone();
            let is_active = current_group.as_ref() == Some(&gid);
            
            return Some(rsx! {
                div {
                    class: if is_active { "flex items-center gap-3 p-3 bg-[var(--bg-elevated)] cursor-pointer border-l-4 border-[var(--color-primary)]" } else { "flex items-center gap-3 p-3 hover:bg-[var(--bg-subtle)] cursor-pointer transition-colors border-l-4 border-transparent" },
                    onclick: move |_| {
                        active_group.set(Some(gid.clone()));
                        target_peer.set(String::new());
                    },
                    div { class: "avatar placeholder",
                        div { class: "bg-[var(--color-secondary)] text-white w-12 rounded-full ring ring-[var(--bg-base)] ring-offset-2 ring-offset-[var(--color-secondary)]",
                            span { class: "text-lg font-bold", "{g.name.chars().next().unwrap_or('G')}" }
                        }
                    }
                    div { class: "flex-1 min-w-0 border-b border-[var(--border-subtle)] pb-3 h-full flex flex-col justify-center",
                        div { class: "flex justify-between items-baseline",
                            span { class: "font-semibold truncate text-[var(--text-main)]", "{g.name}" }
                            span { class: "text-xs text-[var(--text-muted)]", "Unknown date" }
                        }
                        p { class: "text-sm text-[var(--text-muted)] truncate", 
                            "{g.members.len()} members" 
                        }
                    }
                }
            });
        }
        None
    });

    let peer_list_items = peers.iter().filter(|p| **p != local_id).filter_map(|peer| {
        if !query.is_empty() && !peer.to_lowercase().contains(&query) {
            return None;
        }
        let p_clone = peer.clone();
        let is_active = *peer == target && current_group.is_none();
        
        Some(rsx! {
            div {
                class: if is_active { "flex items-center gap-3 p-3 bg-[var(--bg-elevated)] cursor-pointer border-l-4 border-[var(--color-primary)]" } else { "flex items-center gap-3 p-3 hover:bg-[var(--bg-subtle)] cursor-pointer transition-colors border-l-4 border-transparent" },
                onclick: move |_| {
                    target_peer.set(p_clone.clone());
                    active_group.set(None);
                },
                div { class: "avatar placeholder",
                    div { class: "bg-[var(--bg-elevated)] text-[var(--text-main)] w-12 rounded-full border border-[var(--border-subtle)]",
                        span { class: "text-lg font-mono", "{peer.chars().next().unwrap_or('?')}" }
                    }
                }
                div { class: "flex-1 min-w-0 border-b border-[var(--border-subtle)] pb-3 h-full flex flex-col justify-center",
                    div { class: "flex justify-between items-baseline",
                        span { class: "font-semibold truncate text-[var(--text-main)]", "{peer}" }
                    }
                    p { class: "text-sm text-[var(--text-muted)] truncate", "Click to start chatting" }
                }
            }
        })
    });

    let header_name = if let Some(gid) = &current_group {
        if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == *gid) {
             if let DagPayload::Group(g) = &g_node.payload {
                 g.name.clone()
             } else { "Group".to_string() }
        } else { "Loading...".to_string() }
    } else if !target.is_empty() {
        // Maybe fetch profile name?
        target.clone()
    } else {
        String::new()
    };

    rsx! {
        div { class: "flex h-screen bg-[var(--bg-base)] overflow-hidden",
            
            // SIDEBAR
            div { class: "w-[400px] flex flex-col border-r border-[var(--border-default)] bg-[var(--bg-surface)]",
                // Header
                div { class: "h-16 flex items-center justify-between px-4 bg-[var(--bg-surface)] border-b border-[var(--border-default)] flex-shrink-0 z-20",
                    div { class: "avatar",
                        div { class: "w-10 h-10 rounded-full ring ring-primary ring-offset-base-100 ring-offset-2",
                            // User Profile Pic or Default
                             img { src: "https://api.dicebear.com/7.x/identicon/svg?seed={local_id}" }
                        }
                    }
                    div { class: "flex gap-2 text-[var(--text-muted)]",
                        // New Group Button
                        button { 
                            class: "btn btn-circle btn-ghost btn-sm tooltip tooltip-bottom",
                            "data-tip": "New Group",
                            onclick: move |_| show_create_group_modal.set(true),
                            svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { d: "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" }
                            }
                        }
                        // New Chat Button
                        button { 
                            class: "btn btn-circle btn-ghost btn-sm tooltip tooltip-bottom",
                            "data-tip": "New Chat",
                            onclick: move |_| show_new_chat_modal.set(true),
                            svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { d: "M11 5H6a2 2 0 00-2 2v11l5-5h9a2 2 0 002-2v-6a2 2 0 00-2-2h-4" } // Chat bubble-like
                            }
                        }
                        button {
                            class: "btn btn-circle btn-ghost btn-sm",
                             // Menu Icon
                            svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { d: "M12 5v.01M12 12v.01M12 19v.01M12 6a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2zm0 7a1 1 0 110-2 1 1 0 010 2z" }
                            }
                        }
                    }
                }
                
                // Search Bar
                div { class: "p-3 border-b border-[var(--border-default)] bg-[var(--bg-surface)]",
                    div { class: "relative group",
                        div { class: "absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none",
                            svg { class: "h-4 w-4 text-[var(--text-muted)] group-focus-within:text-[var(--color-primary)] transition-colors", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                                path { d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z", stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2" }
                            }
                        }
                        input {
                            class: "input input-sm w-full pl-10 bg-[var(--bg-elevated)] border-none focus:outline-none focus:ring-1 focus:ring-[var(--color-primary)] rounded-full transition-all text-sm",
                            placeholder: "Search chats...",
                            value: "{search_query}",
                            oninput: move |evt| search_query.set(evt.value())
                        }
                    }
                }

                // Chat List
                div { class: "flex-1 overflow-y-auto custom-scrollbar",
                    {group_list_items}
                    {peer_list_items}
                }
            }
            
            // MAIN CHAT AREA
            div { class: "flex-1 flex flex-col min-w-0 bg-[var(--bg-base)]",
                if target.is_empty() && current_group.is_none() {
                    // Empty State
                    div { class: "flex-1 flex flex-col items-center justify-center bg-[var(--bg-deep)] text-[var(--text-muted)] border-b-[6px] border-b-[var(--color-secondary)]",
                         div { class: "text-center max-w-md p-8",
                            div { class: "text-6xl mb-6", "✨" }
                            h1 { class: "text-3xl font-light mb-4 text-[var(--text-main)]", "Welcome to SuperApp Messaging" }
                            p { class: "mb-6 text-lg", "Secure, private, and always available." }
                            div { class: "badge badge-outline", "🔒 End-to-end encrypted" }
                         }
                    }
                } else {
                    // Chat Header
                    div { class: "h-16 flex items-center justify-between px-4 bg-[var(--bg-surface)] border-b border-[var(--border-default)] flex-shrink-0 z-10",
                        div { class: "flex items-center gap-4 cursor-pointer",
                            div { class: "avatar placeholder",
                                div { class: "bg-neutral text-neutral-content w-10 rounded-full",
                                    span { class: "{header_name.chars().next().unwrap_or('?')}" }
                                }
                            }
                             div {
                                h3 { class: "font-semibold text-[var(--text-main)]", "{header_name}" }
                                p { class: "text-xs text-[var(--text-muted)] truncate", 
                                    if current_group.is_some() { "click here for group info" } else { "click for contact info" } 
                                }
                            }
                        }
                        div { class: "flex gap-4 text-[var(--text-muted)]",
                             button { class: "btn btn-ghost btn-circle btn-sm", "🔍" }
                             button { class: "btn btn-ghost btn-circle btn-sm", "⋮" }
                        }
                    }
                    
                    // Messages Area
                    div { class: "flex-1 overflow-y-auto p-4 custom-scrollbar bg-repeat bg-center relative",
                        style: "background-image: radial-gradient(ellipse at top, var(--bg-deep), var(--bg-void));",
                        div { class: "flex flex-col-reverse gap-4 min-h-full justify-end pb-4",
                             {messages_list.into_iter().rev()}
                        }
                    }
                    
                    // Input Area
                    div { class: "min-h-[62px] bg-[var(--bg-surface)] px-4 py-2 flex items-end gap-2 z-20",
                         button { class: "btn btn-ghost btn-circle text-[var(--text-muted)] mb-1", "😊" }
                         
                         div { class: "relative mb-1",
                              button { 
                                  class: "btn btn-ghost btn-circle text-[var(--text-muted)]",
                                  disabled: *is_uploading.read(),
                                  if *is_uploading.read() {
                                    span { class: "loading loading-spinner loading-xs" }
                                  } else {
                                    "📎"
                                  }
                              }
                              input {
                                  class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                  r#type: "file",
                                  onchange: upload_file
                              }
                         }
                         
                         div { class: "flex-1 bg-[var(--input-bg)] rounded-lg flex items-center py-2 px-4 my-1",
                                
                                {
                                    let cmd_tx_input = cmd_tx.clone();
                                    rsx! {
                                        input {
                                            class: "w-full bg-transparent border-none focus:outline-none text-[var(--text-main)]",
                                            placeholder: "Type a message",
                                            value: "{input_msg}",
                                            oninput: move |evt| input_msg.set(evt.value()),
                                            onkeydown: move |evt| {
                                                if evt.key() == Key::Enter {
                                                     let content = input_msg.read().clone();
                                                     let recipient = target_peer.read().clone();
                                                     let group_id = active_group.read().clone();
            
                                                     if !content.is_empty() {
                                                        if let Some(gid) = group_id {
                                                            if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == gid) {
                                                                if let DagPayload::Group(g) = &g_node.payload {
                                                                    for member in &g.members {
                                                                         let _ = cmd_tx_input.send(AppCmd::SendMessage { recipient: member.clone(), content: content.clone(), group_id: Some(gid.clone()) });
                                                                    }
                                                                }
                                                            }
                                                            input_msg.set(String::new());
                                                        } else if !recipient.is_empty() {
                                                            let _ = cmd_tx_input.send(AppCmd::SendMessage { recipient, content, group_id: None });
                                                            input_msg.set(String::new());
                                                        }
                                                     }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                         
                         if !input_msg.read().is_empty() {
                                  
                                  {
                                      let cmd_tx_btn = cmd_tx.clone();
                                      rsx! {
                                          button { 
                                              class: "btn btn-ghost btn-circle text-[var(--text-muted)] mb-1",
                                              onclick: move |_| {
                                                   let content = input_msg.read().clone();
                                                     let recipient = target_peer.read().clone();
                                                     let group_id = active_group.read().clone();
            
                                                     if !content.is_empty() {
                                                        if let Some(gid) = group_id {
                                                            if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == gid) {
                                                                if let DagPayload::Group(g) = &g_node.payload {
                                                                    for member in &g.members {
                                                                         let _ = cmd_tx_btn.send(AppCmd::SendMessage { recipient: member.clone(), content: content.clone(), group_id: Some(gid.clone()) });
                                                                    }
                                                                }
                                                            }
                                                            input_msg.set(String::new());
                                                        } else if !recipient.is_empty() {
                                                            let _ = cmd_tx_btn.send(AppCmd::SendMessage { recipient, content, group_id: None });
                                                            input_msg.set(String::new());
                                                        }
                                                     }
                                              },
                                              svg { class: "w-6 h-6 transform rotate-45 text-[var(--color-primary)]", fill: "currentColor", view_box:"0 0 24 24",
                                                  path { d: "M2.01 21L23 12 2.01 3 2 10l15 2-15 2z" }
                                              }
                                          }
                                      }
                                  }
                         } else {
                              button { class: "btn btn-ghost btn-circle text-[var(--text-muted)] mb-1", "🎤" }
                         }
                    }
                }
            }
        }
        
        // NEW CHAT MODAL
        if *show_new_chat_modal.read() {
            div { class: "fixed inset-0 bg-black/60 z-50 flex items-center justify-center animate-fade-in backdrop-blur-sm",
                div { class: "bg-[var(--bg-surface)] w-[400px] h-[500px] flex flex-col rounded-xl shadow-2xl border border-[var(--border-subtle)] overflow-hidden",
                    // Header
                    div { class: "h-16 px-6 bg-[var(--bg-surface)] flex items-center gap-4 border-b border-[var(--border-default)]",
                        button { class: "btn btn-ghost btn-circle btn-sm", onclick: move |_| show_new_chat_modal.set(false), 
                             svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { d: "M15 19l-7-7 7-7" } }
                        }
                        div { class: "flex flex-col",
                             span { class: "font-semibold text-lg", "New Message" }
                             span { class: "text-xs text-[var(--text-muted)]", "Select a contact to start chatting" }
                        }
                    }
                    
                    // Search
                    div { class: "p-4 border-b border-[var(--border-subtle)] bg-[var(--bg-elevated)]",
                         input {
                            class: "input input-sm w-full bg-[var(--bg-base)] border-none focus:ring-1 focus:ring-[var(--color-primary)] rounded-lg",
                            placeholder: "Search contacts..."
                        }
                    }
                    
                    // Options
                    div { class: "flex-1 overflow-y-auto custom-scrollbar p-2",
                        div { class: "px-4 py-3 text-[var(--color-primary)] text-xs font-bold tracking-wider opacity-80", "CONTACTS" }
                        
                        {
                            app_state.peers.read().iter().filter(|p| **p != local_id).map(|peer| {
                                let p_clone = peer.clone();
                                rsx! {
                                    div {
                                        class: "flex items-center gap-4 p-3 hover:bg-[var(--bg-subtle)] cursor-pointer rounded-lg transition-colors m-1",
                                        onclick: move |_| {
                                            target_peer.set(p_clone.clone());
                                            active_group.set(None);
                                            show_new_chat_modal.set(false);
                                        },
                                        div { class: "avatar placeholder",
                                            div { class: "bg-[var(--bg-deep)] text-[var(--text-muted)] w-10 rounded-full border border-[var(--border-subtle)]",
                                                span { class: "text-xs font-mono", "{peer.get(0..2).unwrap_or(\"??\")}" }
                                            }
                                        }
                                        div { class: "flex-1 min-w-0",
                                            p { class: "font-medium truncate text-sm", "{peer}" }
                                            p { class: "text-xs text-[var(--text-muted)] truncate", "Network Peer" }
                                        }
                                    }
                                }
                            })
                        }
                    }
                }
            }
        }
        
        // CREATE GROUP MODAL
        if *show_create_group_modal.read() {
             div { class: "fixed inset-0 bg-black/60 z-50 flex items-center justify-center animate-fade-in backdrop-blur-sm",
                div { class: "bg-[var(--bg-surface)] w-[400px] max-h-[600px] flex flex-col rounded-xl shadow-2xl border border-[var(--border-subtle)] overflow-hidden",
                     // Header
                    div { class: "h-16 px-6 bg-[var(--bg-surface)] flex items-center gap-4 border-b border-[var(--border-default)]",
                        button { class: "btn btn-ghost btn-circle btn-sm", onclick: move |_| show_create_group_modal.set(false), 
                             svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { d: "M15 19l-7-7 7-7" } }
                        }
                        span { class: "font-semibold text-lg", "Create Group" }
                    }
                    
                    div { class: "p-6 flex flex-col gap-6",
                         // Group Image (Mock)
                         div { class: "flex justify-center",
                             div { class: "w-24 h-24 rounded-full bg-[var(--bg-elevated)] flex items-center justify-center cursor-pointer hover:opacity-80 transition-opacity border-2 border-dashed border-[var(--border-default)]",
                                 span { class: "text-3xl", "📷" }
                             }
                         }
                         
                         // Subject
                         div { class: "form-control",
                             input {
                                 class: "input bg-[var(--bg-elevated)] w-full focus:ring-1 focus:ring-[var(--color-primary)] border border-[var(--border-default)] rounded-lg",
                                 placeholder: "Group Subject (Required)",
                                 value: "{new_group_name}",
                                 oninput: move |evt| new_group_name.set(evt.value())
                             }
                         }
                    }
                     
                    div { class: "px-6 py-2 text-[var(--text-muted)] text-xs font-bold tracking-wider uppercase", "PARTICIPANTS" }
                    
                    div { class: "flex-1 overflow-y-auto px-4 custom-scrollbar",
                         {
                            app_state.peers.read().iter().filter(|p| **p != local_id).map(|peer| {
                                let p_clone = peer.clone();
                                let is_selected = selected_peers_for_group.read().contains(peer);
                                rsx! {
                                    div { 
                                        class: "flex items-center gap-3 p-3 hover:bg-[var(--bg-subtle)] cursor-pointer rounded-lg m-1 transition-colors",
                                        onclick: move |_| {
                                            let mut set = selected_peers_for_group.read().clone();
                                            if set.contains(&p_clone) {
                                                set.remove(&p_clone);
                                            } else {
                                                set.insert(p_clone.clone());
                                            }
                                            selected_peers_for_group.set(set);
                                        },
                                        div { class: "avatar placeholder",
                                            div { class: "bg-[var(--bg-elevated)] text-[var(--text-main)] w-8 rounded-full border border-[var(--border-subtle)]",
                                                span { class: "text-xs", "{peer.get(0..1).unwrap_or(\"?\")}" }
                                            }
                                        }
                                        span { class: "flex-1 truncate text-sm font-medium", "{peer}" }
                                        input { type: "checkbox", class: "checkbox checkbox-primary checkbox-sm rounded", checked: is_selected, readonly: true }
                                    }
                                }
                            })
                        }
                    }
                    
                    div { class: "p-4 bg-[var(--bg-elevated)] flex justify-center border-t border-[var(--border-default)]",
                        {
                             let cmd_tx_create = cmd_tx.clone();
                             rsx! {
                                 button { 
                                     class: "btn btn-circle btn-primary shadow-lg hover:scale-105 transition-transform",
                                     disabled: new_group_name.read().is_empty() || selected_peers_for_group.read().is_empty(),
                                     onclick: move |_| {
                                         let name = new_group_name.read().clone();
                                         let members: Vec<String> = selected_peers_for_group.read().iter().cloned().collect();
                                         if !name.is_empty() && !members.is_empty() {
                                             cmd_tx_create.send(AppCmd::CreateGroup { name, members }).unwrap();
                                             show_create_group_modal.set(false);
                                             new_group_name.set(String::new());
                                             selected_peers_for_group.set(std::collections::HashSet::new());
                                         }
                                     },
                                     svg { class: "w-6 h-6", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "3",
                                        path { d: "M5 13l4 4L19 7" }
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
fn MessageItem(msg: crate::backend::dag::DagNode, is_me: bool, content: String) -> Element {
    // Check for file pattern: [FILE:cid:key:nonce:mime:filename]
    let file_data = if content.starts_with("[FILE:") && content.ends_with("]") {
        let parts: Vec<&str> = content.trim_start_matches("[FILE:").trim_end_matches("]").split(':').collect();
        if parts.len() >= 5 {
            Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string(), parts[3].to_string(), parts[4].to_string()))
        } else {
            None
        }
    } else {
        None
    };

    rsx! {
        div {
            class: if is_me { "flex justify-end mb-2 animate-slide-in-right" } else { "flex justify-start mb-2 animate-slide-in-left" },
            div {
                class: if is_me { "bg-[var(--color-primary)] text-white rounded-2xl rounded-tr-none p-3 max-w-[70%] shadow-lg relative group" } else { "bg-[var(--bg-elevated)] text-[var(--text-main)] rounded-2xl rounded-tl-none p-3 max-w-[70%] shadow-lg border border-[var(--border-subtle)] relative group" },
                if let Some((cid, key, nonce, mime, filename)) = file_data {
                    FileAttachment { cid, encryption_key: key, nonce, mime, filename }
                } else {
                    span { class: "break-words text-sm leading-relaxed", "{content}" }
                }
                div { class: "text-[10px] opacity-70 text-right mt-1 flex items-center justify-end gap-1",
                    "{msg.timestamp}"
                    if is_me {
                         span { class: "font-bold", "✓✓" } // Read receipts style
                    }
                }
            }
        }
    }
}

#[component]
fn FileAttachment(cid: String, encryption_key: String, nonce: String, mime: String, filename: String) -> Element {

    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    
    let cache = app_state.blob_cache.read();
    let blob_data = cache.get(&cid).cloned();
    drop(cache);
    
    let mut decrypted_url = use_signal(|| None::<String>);
    let mut is_decrypting = use_signal(|| false);

    // Fetch if needed
    let cid_clone = cid.clone();
    use_effect(move || {
        let app_state = app_state.clone();
        let cmd_tx = cmd_tx.clone();
        if !app_state.blob_cache.read().contains_key(&cid_clone) {
             let _ = cmd_tx.send(AppCmd::FetchBlock { cid: cid_clone.clone(), peer_id: None });
        }
    });

    let mime_decrypt = mime.clone();

    // Decrypt when data is available
    use_effect(move || {
        if let Some(data_uri) = blob_data.clone() {
            if decrypted_url.read().is_none() && !*is_decrypting.read() {
                is_decrypting.set(true);
                if let Some(base64_part) = data_uri.split(',').nth(1) {
                    if let Ok(encrypted_bytes) = general_purpose::STANDARD.decode(base64_part) {
                        if let Ok(key_bytes) = hex::decode(&encryption_key) {
                             if let Ok(nonce_bytes) = hex::decode(&nonce) {
                                  if key_bytes.len() == 32 && nonce_bytes.len() == 12 {
                                      let key_arr = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
                                      let cipher = Aes256Gcm::new(key_arr);
                                      let nonce_arr = aes_gcm::aead::generic_array::GenericArray::from_slice(&nonce_bytes);
                                      
                                      if let Ok(plaintext) = cipher.decrypt(nonce_arr, encrypted_bytes.as_ref()) {
                                          let b64_plain = general_purpose::STANDARD.encode(&plaintext);
                                          let final_url = format!("data:{};base64,{}", mime_decrypt, b64_plain);
                                          decrypted_url.set(Some(final_url));
                                      }
                                  }
                             }
                        }
                    }
                }
                is_decrypting.set(false);
            }
        }
    });

    let filename_download = filename.clone();
    let mime_image_render = mime.clone();

    rsx! {
        div { class: "flex items-center gap-3 p-2 bg-black/5 rounded",
            div { class: "text-2xl", "📄" }
            div { class: "flex-1 min-w-0",
                p { class: "font-semibold truncate text-sm", "{filename}" }
                p { class: "text-xs opacity-70", "Encrypted File" }
            }
            if let Some(url) = decrypted_url.read().clone() {
                a { 
                    class: "btn btn-xs btn-ghost",
                    href: "{url}",
                    download: "{filename_download}",
                    "⬇️"
                }
            } else if *is_decrypting.read() {
                span { class: "loading loading-spinner loading-xs" }
            } else {
                button { class: "btn btn-xs btn-ghost", "Decrypting..." }
            }
        }
        if let Some(url) = decrypted_url.read().clone() {
             if mime_image_render.starts_with("image/") {
                 img { src: "{url}", class: "mt-2 rounded max-w-full h-auto max-h-64 object-contain" }
             }
        }
    }
}