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
    let mut target_peer = use_signal(|| String::new());
    let mut active_group = use_signal(|| Option::<String>::None); // Group ID
    
    let mut show_create_group = use_signal(|| false);
    let mut new_group_name = use_signal(|| String::new());
    let mut selected_peers = use_signal(|| std::collections::HashSet::<String>::new());
    
    // File upload state
    let mut is_uploading = use_signal(|| false);
    let mut last_uploaded_file_info = use_signal(|| None::<(String, String, String, String, String)>); // (cid, key, nonce, mime, filename)
    
    // Watch for blob creation to match with our upload
    let mut pending_upload = use_signal(|| None::<(String, String, String, String)>); // (key, nonce, mime, filename)

    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    
    let target = target_peer.read().clone();
    let current_group = active_group.read().clone();
    let local_id = app_state.local_peer_id.read().clone();

    // Fetch messages when target changes
    let cmd_tx_effect = cmd_tx.clone();
    let cmd_tx_effect2 = cmd_tx.clone();
    let target_effect = target.clone();
    let group_effect = current_group.clone();
    
    use_effect(move || {
        let t = target_effect.clone();
        if !t.is_empty() {
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
                // Determine if this blob_id corresponds to our upload?
                // last_created_blob is global, so racing is possible but unlikely in single user context.
                // We assume it's ours if we have a pending upload.
                
                // Format: [FILE:cid:key:nonce:mime:filename]
                let info = (blob_id.clone(), key, nonce, mime, filename);
                last_uploaded_file_info.set(Some(info));
                pending_upload.set(None);
                is_uploading.set(false);
            }
        }
    });

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
                         // 1. Generate Key & Nonce
                         let key = Aes256Gcm::generate_key(&mut OsRng);
                         let cipher = Aes256Gcm::new(&key);
                         let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits
                         
                         // 2. Encrypt
                         if let Ok(ciphertext) = cipher.encrypt(&nonce, file_bytes.as_ref()) {
                             // 3. Encode as Base64 for BlobPayload
                             let data_base64 = general_purpose::STANDARD.encode(&ciphertext);
                             
                             // 4. Publish Blob (Encrypted)
                             // We use "application/encrypted" as public MIME type to hide metadata
                             let _ = cmd_tx.send(AppCmd::PublishBlob { mime_type: "application/encrypted".to_string(), data: data_base64 });
                             
                             // 5. Store pending info to match with CID later
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

    // If we just finished an upload, append to input
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

    // Prepare message list - logic depends on if we are in group or DM
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
                // Only show 1-on-1 messages that are NOT part of a group here
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

    let group_name = if let Some(gid) = &current_group {
        if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == *gid) {
             if let DagPayload::Group(g) = &g_node.payload {
                 g.name.clone()
             } else { "Group".to_string() }
        } else { "Loading...".to_string() }
    } else { "Chat".to_string() };

    rsx! { 
        div { class: "flex flex-col min-h-screen",
            div { class: "flex-1 page-container py-8 animate-fade-in",
                div { class: "flex gap-6 h-[calc(100vh-128px)]",
                    
                    // Sidebar
                    div { class: "w-80 flex-shrink-0 panel flex flex-col",
                        div { class: "panel-header flex justify-between items-center",
                            h2 { class: "panel-title", "Messages" }
                            button { 
                                class: "btn btn-sm btn-ghost", 
                                onclick: move |_| show_create_group.set(true),
                                "+" 
                            }
                        }
                        
                        // Identity card
                        div { class: "card mb-4",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "label", "Your Identity" }
                                span { class: "w-2 h-2 rounded-full bg-green-500" }
                            }
                            p { class: "text-xs font-mono text-[var(--text-muted)] truncate", "{local_id}" }
                        }
                        
                        // Groups List
                        div { class: "flex-none mb-4",
                             p { class: "label mb-2", "Groups" }
                             {
                                 app_state.groups.read().iter().map(|group_node| {
                                     let group = if let DagPayload::Group(g) = &group_node.payload { g } else { return rsx!{} };
                                     let gid = group_node.id.clone();
                                     let is_active = current_group.as_ref() == Some(&gid);
                                     
                                     rsx! {
                                         div {
                                             class: if is_active { "list-item active" } else { "list-item" },
                                             onclick: move |_| {
                                                 active_group.set(Some(gid.clone()));
                                                 target_peer.set(String::new()); // Clear DM selection
                                             },
                                             div { class: "avatar avatar-sm bg-indigo-500 text-white", "G" }
                                             div { class: "list-item-content",
                                                 p { class: "list-item-title truncate", "{group.name}" }
                                                 p { class: "list-item-subtitle", "{group.members.len()} members" }
                                             }
                                         }
                                     }
                                 })
                             }
                        }

                        // Search/Direct List
                        div { class: "form-group",
                            input {
                                class: "input",
                                placeholder: "Enter Peer ID...",
                                value: "{target_peer}",
                                oninput: move |evt| {
                                     target_peer.set(evt.value());
                                     active_group.set(None); // Clear group selection
                                }
                            }
                        }
                        
                        // Peer list
                        div { class: "flex-1 overflow-y-auto",
                            p { class: "label mb-3", "Direct Messages" }
                            {
                                app_state.peers.read().iter()
                                    .filter(|peer| **peer != local_id)
                                    .map(|peer| {
                                        let peer_clone = peer.clone();
                                        let is_active = *peer == target && current_group.is_none();
                                        
                                        rsx! {
                                            div {
                                                class: if is_active { "list-item active" } else { "list-item" },
                                                onclick: move |_| {
                                                    target_peer.set(peer_clone.clone());
                                                    active_group.set(None);
                                                },
                                                div { class: "avatar avatar-sm",
                                                    "{peer.get(0..2).unwrap_or(\"??\")}"
                                                }
                                                div { class: "list-item-content",
                                                    p { class: "list-item-title truncate", "{peer}" }
                                                    p { class: "list-item-subtitle", "Click to chat" }
                                                }
                                            }
                                        }
                                    })
                            }
                        }
                    }
                    
                    // Chat area
                    div { class: "flex-1 panel flex flex-col overflow-hidden",
                        if target.is_empty() && current_group.is_none() {
                            div { class: "flex-1 empty-state",
                                div { class: "empty-state-icon", "💬" }
                                p { class: "empty-state-title", "Select a chat" }
                                p { class: "empty-state-text", "Select a group or peer to start chatting" }
                            }
                        } else {
                            // Chat header
                            div { class: "p-4 border-b border-[var(--border-default)] flex items-center gap-3",
                                div { class: "avatar" }
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2",
                                        p { class: "font-semibold", 
                                            "{group_name}" 
                                        }
                                        span { class: "badge badge-online text-xs", "🔒 Encrypted" }
                                    }
                                    p { class: "text-xs text-[var(--text-muted)] font-mono truncate", 
                                        if let Some(gid) = &current_group { "{gid}" } else { "{target}" } 
                                    }
                                }
                            }

                            // Messages
                            div { class: "flex-1 p-4 overflow-y-auto flex flex-col-reverse gap-3",
                                {messages_list.into_iter().rev()}
                            }
                            
                            // Input
                            div { class: "p-4 border-t border-[var(--border-default)]",
                                div { class: "flex gap-3",
                                    {
                                        let cmd_tx_key = cmd_tx.clone();
                                        rsx! {
                                            input {
                                                class: "input flex-1",
                                                placeholder: "Type a message...",
                                                value: "{input_msg}",
                                                oninput: move |evt| input_msg.set(evt.value()),
                                                onkeydown: move |evt| {
                                                    if evt.key() == Key::Enter {
                                                        let content = input_msg.read().clone();
                                                        let recipient = target_peer.read().clone();
                                                        let group_id = active_group.read().clone();

                                                        if !content.is_empty() {
                                                            if let Some(gid) = group_id {
                                                                // Fan-out to group members!
                                                                // Implementation Detail: Ideally Backend handles fan-out.
                                                                // But our backend SendMessage takes 1 recipient.
                                                                // Actually SendMessage with group_id should handle iteration?
                                                                // Wait, backend's SendMessage takes `recipient: String`.
                                                                // We need to iterate HERE in frontend or make Backend smarter.
                                                                // Let's iterate here for MVP as planned.
                                                                if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == gid) {
                                                                    if let DagPayload::Group(g) = &g_node.payload {
                                                                        for member in &g.members {
                                                                             // Don't send to self via network, but maybe we should store it?
                                                                             // Actually backend loop handles storing.
                                                                             // Wait, backend `SendMessage` does store "message:v1".
                                                                             // If we send to self, it works.
                                                                             cmd_tx_key.send(AppCmd::SendMessage { recipient: member.clone(), content: content.clone(), group_id: Some(gid.clone()) }).unwrap();
                                                                        }
                                                                    }
                                                                }
                                                                input_msg.set(String::new());
                                                            } else if !recipient.is_empty() {
                                                                cmd_tx_key.send(AppCmd::SendMessage { recipient, content, group_id: None }).unwrap();
                                                                input_msg.set(String::new());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        let cmd_tx_click = cmd_tx.clone();
                                        rsx! {
                                            div { class: "relative",
                                                button {
                                                    class: "btn btn-secondary",
                                                    disabled: *is_uploading.read(),
                                                    if *is_uploading.read() {
                                                        "⏳"
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
                                            button {
                                                class: "btn btn-primary",
                                                onclick: move |_| {
                                                    let content = input_msg.read().clone();
                                                    let recipient = target_peer.read().clone();
                                                    let group_id = active_group.read().clone();
                                                    
                                                     if !content.is_empty() {
                                                        if let Some(gid) = group_id {
                                                            if let Some(g_node) = app_state.groups.read().iter().find(|n| n.id == gid) {
                                                                if let DagPayload::Group(g) = &g_node.payload {
                                                                    for member in &g.members {
                                                                         cmd_tx_click.send(AppCmd::SendMessage { recipient: member.clone(), content: content.clone(), group_id: Some(gid.clone()) }).unwrap();
                                                                    }
                                                                }
                                                            }
                                                            input_msg.set(String::new());
                                                        } else if !recipient.is_empty() {
                                                            cmd_tx_click.send(AppCmd::SendMessage { recipient, content, group_id: None }).unwrap();
                                                            input_msg.set(String::new());
                                                        }
                                                    }
                                                },
                                                "Send"
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
        
        // CREATE GROUP MODAL
        if *show_create_group.read() {
             div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                div { class: "panel w-96 max-h-[80vh] flex flex-col",
                    div { class: "panel-header flex justify-between",
                        h3 { class: "panel-title", "Create New Group" }
                        button { class: "btn btn-sm btn-ghost", onclick: move |_| show_create_group.set(false), "✕" }
                    }
                    div { class: "p-4 flex-1 overflow-y-auto",
                        div { class: "form-group",
                            label { class: "label", "Group Name" }
                            input {
                                class: "input",
                                value: "{new_group_name}",
                                oninput: move |evt| new_group_name.set(evt.value())
                            }
                        }
                        div { class: "form-group mt-4",
                            label { class: "label", "Select Members" }
                            div { class: "max-h-40 overflow-y-auto border border-[var(--border-default)] rounded p-2",
                                {
                                    app_state.peers.read().iter().filter(|p| **p != local_id).map(|peer| {
                                        let p_clone = peer.clone();
                                        let is_selected = selected_peers.read().contains(peer);
                                        rsx! {
                                            div { 
                                                class: "flex items-center gap-2 p-2 hover:bg-[var(--bg-subtle)] cursor-pointer",
                                                onclick: move |_| {
                                                    let mut set = selected_peers.read().clone();
                                                    if set.contains(&p_clone) {
                                                        set.remove(&p_clone);
                                                    } else {
                                                        set.insert(p_clone.clone());
                                                    }
                                                    selected_peers.set(set);
                                                },
                                                input { type: "checkbox", checked: is_selected, readonly: true }
                                                span { class: "text-sm truncate", "{peer}" }
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }
                    div { class: "p-4 border-t border-[var(--border-default)] flex justify-end gap-2",
                        button { class: "btn btn-secondary", onclick: move |_| show_create_group.set(false), "Cancel" }
                        button { 
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let name = new_group_name.read().clone();
                                let members: Vec<String> = selected_peers.read().iter().cloned().collect();
                                if !name.is_empty() && !members.is_empty() {
                                    cmd_tx.send(AppCmd::CreateGroup { name, members }).unwrap();
                                    show_create_group.set(false);
                                    new_group_name.set(String::new());
                                    selected_peers.set(std::collections::HashSet::new());
                                }
                            },
                            "Create Group"
                        }
                    }
                }
             }
        }
    }
}

#[component]
fn MessageItem(msg: crate::backend::dag::DagNode, is_me: bool, content: String) -> Element {
    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();

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
            class: if is_me { "flex flex-col items-end" } else { "flex flex-col items-start" },
            div {
                class: if is_me { "message-bubble sent" } else { "message-bubble received" },
                if let Some((cid, key, nonce, mime, filename)) = file_data {
                    FileAttachment { cid, encryption_key: key, nonce, mime, filename }
                } else {
                    "{content}"
                }
            }
            span { class: "message-time", "{msg.timestamp}" }
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
                // Data URI format: data:mime;base64,data
                // But wait, our backend stores it as data:mime;base64,data
                // And we stored "application/encrypted" as mime!
                
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
    let mime_image_check = mime.clone();
    let mime_image_render = mime.clone();

    rsx! {
        div { class: "flex items-center gap-3 p-2 bg-black/20 rounded",
            div { class: "text-2xl", "📄" }
            div { class: "flex-1 min-w-0",
                p { class: "font-semibold truncate text-sm", "{filename}" }
                p { class: "text-xs opacity-70", "Encrypted File" }
            }
            if let Some(url) = decrypted_url.read().clone() {
                a { 
                    class: "btn btn-xs btn-primary",
                    href: "{url}",
                    download: "{filename_download}",
                    "Download"
                }
                if mime_image_check.starts_with("image/") {
                     // Auto-show image if small enough? Or just a button?
                     // Let's safe-render it as an image below download button if user wants
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