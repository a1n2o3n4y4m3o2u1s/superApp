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
    let mut active_group = use_signal(|| Option::<String>::None);
    let mut target_peer = use_signal(|| String::new());

    // Modals
    let mut show_new_chat_modal = use_signal(|| false);
    let mut show_create_group_modal = use_signal(|| false);
    
    // New Group Form
    let mut new_group_name = use_signal(|| String::new());
    let mut selected_peers_for_group = use_signal(|| std::collections::HashSet::<String>::new());
    
    // New Chat Form - direct peer ID input
    let mut new_chat_peer_id = use_signal(|| String::new());

    // Search
    let mut search_query = use_signal(|| String::new());
    
    // File upload state
    let mut is_uploading = use_signal(|| false);
    let mut last_uploaded_file_info = use_signal(|| None::<(String, String, String, String, String)>);
    let mut pending_upload = use_signal(|| None::<(String, String, String, String)>);

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
             *viewed_profile.write() = None;
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
    
    let query = search_query.read().to_lowercase();
    
    let group_list_items = groups.iter().filter_map(|group_node| {
        if let DagPayload::Group(g) = &group_node.payload {
            if !query.is_empty() && !g.name.to_lowercase().contains(&query) {
                return None;
            }
            let gid = group_node.id.clone();
            let is_active = current_group.as_ref() == Some(&gid);
            let bg_class = if is_active { "chat-item chat-item-active" } else { "chat-item" };
            
            return Some(rsx! {
                div {
                    class: "{bg_class}",
                    onclick: move |_| {
                        active_group.set(Some(gid.clone()));
                        target_peer.set(String::new());
                    },
                    div { class: "chat-avatar chat-avatar-group",
                        span { "{g.name.chars().next().unwrap_or('G')}" }
                    }
                    div { class: "chat-info",
                        div { class: "chat-name", "{g.name}" }
                        div { class: "chat-preview", "{g.members.len()} members" }
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
        let bg_class = if is_active { "chat-item chat-item-active" } else { "chat-item" };
        let short_id = if peer.len() > 12 { format!("{}...", &peer[0..12]) } else { peer.clone() };
        
        Some(rsx! {
            div {
                class: "{bg_class}",
                onclick: move |_| {
                    target_peer.set(p_clone.clone());
                    active_group.set(None);
                },
                div { class: "chat-avatar",
                    span { "{peer.chars().next().unwrap_or('?')}" }
                }
                div { class: "chat-info",
                    div { class: "chat-name", "{short_id}" }
                    div { class: "chat-preview", "Click to chat" }
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
        if target.len() > 20 { format!("{}...", &target[0..20]) } else { target.clone() }
    } else {
        String::new()
    };

    rsx! {
        style { {MESSAGING_CSS} }
        
        div { class: "messaging-container",
            
            // SIDEBAR
            div { class: "messaging-sidebar",
                // Header
                div { class: "sidebar-header",
                    div { class: "sidebar-title", "Messages" }
                    div { class: "sidebar-actions",
                        button { 
                            class: "icon-btn",
                            title: "New Group",
                            onclick: move |_| show_create_group_modal.set(true),
                            "👥"
                        }
                        button { 
                            class: "icon-btn",
                            title: "New Chat",
                            onclick: move |_| show_new_chat_modal.set(true),
                            "✉️"
                        }
                    }
                }
                
                // Search Bar
                div { class: "sidebar-search",
                    input {
                        class: "search-input",
                        placeholder: "Search chats...",
                        value: "{search_query}",
                        oninput: move |evt| search_query.set(evt.value())
                    }
                }

                // Chat List
                div { class: "chat-list",
                    if groups.is_empty() && peers.len() <= 1 {
                        div { class: "empty-chats",
                            div { class: "empty-icon", "💬" }
                            div { class: "empty-text", "No chats yet" }
                            div { class: "empty-hint", "Click ✉️ to start a new chat" }
                        }
                    } else {
                        {group_list_items}
                        {peer_list_items}
                    }
                }
            }
            
            // MAIN CHAT AREA
            div { class: "chat-area",
                if target.is_empty() && current_group.is_none() {
                    // Empty State
                    div { class: "chat-empty-state",
                        div { class: "empty-state-icon", "💬" }
                        h2 { class: "empty-state-title", "Welcome to Messaging" }
                        p { class: "empty-state-text", "Select a chat or start a new conversation" }
                        button { 
                            class: "btn-primary",
                            onclick: move |_| show_new_chat_modal.set(true),
                            "Start New Chat"
                        }
                    }
                } else {
                    // Chat Header
                    div { class: "chat-header",
                        div { class: "chat-header-info",
                            div { class: "chat-header-avatar",
                                "{header_name.chars().next().unwrap_or('?')}"
                            }
                            div { class: "chat-header-details",
                                div { class: "chat-header-name", "{header_name}" }
                                div { class: "chat-header-status", 
                                    if current_group.is_some() { "Group Chat" } else { "Direct Message" } 
                                }
                            }
                        }
                    }
                    
                    // Messages Area
                    div { class: "messages-area",
                        div { class: "messages-container",
                            {messages_list.into_iter().rev()}
                        }
                    }
                    
                    // Input Area
                    div { class: "message-input-area",
                        div { class: "input-row",
                            div { class: "file-upload-btn",
                                if *is_uploading.read() {
                                    span { class: "uploading", "⏳" }
                                } else {
                                    span { "📎" }
                                }
                                input {
                                    r#type: "file",
                                    class: "file-input-hidden",
                                    onchange: upload_file
                                }
                            }
                            
                            {
                                let cmd_tx_input = cmd_tx.clone();
                                rsx! {
                                    input {
                                        class: "message-input",
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
                            
                            {
                                let cmd_tx_btn = cmd_tx.clone();
                                let has_content = !input_msg.read().is_empty();
                                rsx! {
                                    button { 
                                        class: if has_content { "send-btn send-btn-active" } else { "send-btn" },
                                        disabled: !has_content,
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
                                        "➤"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // NEW CHAT MODAL
        if *show_new_chat_modal.read() {
            div { 
                class: "modal-overlay",
                onclick: move |_| show_new_chat_modal.set(false),
                
                div { 
                    class: "modal-content",
                    onclick: move |evt| evt.stop_propagation(),
                    
                    // Modal Header with X button
                    div { class: "modal-header",
                        h3 { class: "modal-title", "Start New Chat" }
                        button { 
                            class: "modal-close-btn",
                            onclick: move |_| show_new_chat_modal.set(false),
                            "✕"
                        }
                    }
                    
                    // Modal Body
                    div { class: "modal-body",
                        // Direct Peer ID Input
                        div { class: "input-group",
                            label { class: "input-label", "Enter Peer ID" }
                            input {
                                class: "modal-input",
                                placeholder: "Paste peer ID here...",
                                value: "{new_chat_peer_id}",
                                oninput: move |evt| new_chat_peer_id.set(evt.value())
                            }
                            p { class: "input-hint", "You can get someone's peer ID from their profile page" }
                        }
                        
                        button {
                            class: "btn-primary btn-full",
                            disabled: new_chat_peer_id.read().is_empty(),
                            onclick: move |_| {
                                let peer_id = new_chat_peer_id.read().clone().trim().to_string();
                                if !peer_id.is_empty() {
                                    target_peer.set(peer_id);
                                    active_group.set(None);
                                    show_new_chat_modal.set(false);
                                    new_chat_peer_id.set(String::new());
                                }
                            },
                            "Start Chat"
                        }
                        
                        // Known Peers Section
                        if !peers.is_empty() {
                            div { class: "section-divider",
                                span { "or select from network" }
                            }
                            
                            div { class: "peer-list",
                                {
                                    app_state.peers.read().iter().filter(|p| **p != local_id).map(|peer| {
                                        let p_clone = peer.clone();
                                        let short_id = if peer.len() > 16 { format!("{}...", &peer[0..16]) } else { peer.clone() };
                                        rsx! {
                                            div {
                                                class: "peer-item",
                                                onclick: move |_| {
                                                    target_peer.set(p_clone.clone());
                                                    active_group.set(None);
                                                    show_new_chat_modal.set(false);
                                                },
                                                div { class: "peer-avatar", "{peer.chars().next().unwrap_or('?')}" }
                                                div { class: "peer-info",
                                                    div { class: "peer-name", "{short_id}" }
                                                    div { class: "peer-label", "Network Peer" }
                                                }
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // CREATE GROUP MODAL
        if *show_create_group_modal.read() {
            div { 
                class: "modal-overlay",
                onclick: move |_| show_create_group_modal.set(false),
                
                div { 
                    class: "modal-content modal-content-large",
                    onclick: move |evt| evt.stop_propagation(),
                    
                    // Modal Header with X button
                    div { class: "modal-header",
                        h3 { class: "modal-title", "Create Group" }
                        button { 
                            class: "modal-close-btn",
                            onclick: move |_| show_create_group_modal.set(false),
                            "✕"
                        }
                    }
                    
                    // Modal Body
                    div { class: "modal-body",
                        // Group Name Input
                        div { class: "input-group",
                            label { class: "input-label", "Group Name" }
                            input {
                                class: "modal-input",
                                placeholder: "Enter group name...",
                                value: "{new_group_name}",
                                oninput: move |evt| new_group_name.set(evt.value())
                            }
                        }
                        
                        // Add Member by ID
                        div { class: "input-group",
                            label { class: "input-label", "Add Member by Peer ID" }
                            div { class: "add-member-row",
                                input {
                                    class: "modal-input",
                                    id: "add-member-input",
                                    placeholder: "Paste peer ID...",
                                }
                                button {
                                    class: "btn-secondary",
                                    onclick: move |_| {
                                        // Get value from DOM - simplified approach
                                        // In production, use proper signal
                                    },
                                    "Add"
                                }
                            }
                        }
                        
                        // Selected Members
                        if !selected_peers_for_group.read().is_empty() {
                            div { class: "input-group",
                                label { class: "input-label", "Selected Members ({selected_peers_for_group.read().len()})" }
                                div { class: "selected-members",
                                    {
                                        selected_peers_for_group.read().iter().map(|peer| {
                                            let p_clone = peer.clone();
                                            let short_id = if peer.len() > 12 { format!("{}...", &peer[0..12]) } else { peer.clone() };
                                            rsx! {
                                                div { class: "selected-member-chip",
                                                    span { "{short_id}" }
                                                    button {
                                                        class: "chip-remove",
                                                        onclick: move |_| {
                                                            let mut set = selected_peers_for_group.read().clone();
                                                            set.remove(&p_clone);
                                                            selected_peers_for_group.set(set);
                                                        },
                                                        "✕"
                                                    }
                                                }
                                            }
                                        })
                                    }
                                }
                            }
                        }
                        
                        // Network Peers to Add
                        if !peers.is_empty() {
                            div { class: "section-divider",
                                span { "Add from network" }
                            }
                            
                            div { class: "peer-list",
                                {
                                    app_state.peers.read().iter().filter(|p| **p != local_id && !selected_peers_for_group.read().contains(*p)).map(|peer| {
                                        let p_clone = peer.clone();
                                        let short_id = if peer.len() > 16 { format!("{}...", &peer[0..16]) } else { peer.clone() };
                                        rsx! {
                                            div {
                                                class: "peer-item",
                                                onclick: move |_| {
                                                    let mut set = selected_peers_for_group.read().clone();
                                                    set.insert(p_clone.clone());
                                                    selected_peers_for_group.set(set);
                                                },
                                                div { class: "peer-avatar", "{peer.chars().next().unwrap_or('?')}" }
                                                div { class: "peer-info",
                                                    div { class: "peer-name", "{short_id}" }
                                                    div { class: "peer-label", "Click to add" }
                                                }
                                                span { class: "add-indicator", "+" }
                                            }
                                        }
                                    })
                                }
                            }
                        }
                    }
                    
                    // Modal Footer
                    div { class: "modal-footer",
                        button {
                            class: "btn-secondary",
                            onclick: move |_| {
                                show_create_group_modal.set(false);
                                new_group_name.set(String::new());
                                selected_peers_for_group.set(std::collections::HashSet::new());
                            },
                            "Cancel"
                        }
                        {
                            let cmd_tx_create = cmd_tx.clone();
                            let can_create = !new_group_name.read().is_empty() && !selected_peers_for_group.read().is_empty();
                            rsx! {
                                button { 
                                    class: "btn-primary",
                                    disabled: !can_create,
                                    onclick: move |_| {
                                        let name = new_group_name.read().clone();
                                        let members: Vec<String> = selected_peers_for_group.read().iter().cloned().collect();
                                        if !name.is_empty() && !members.is_empty() {
                                            let _ = cmd_tx_create.send(AppCmd::CreateGroup { name, members });
                                            show_create_group_modal.set(false);
                                            new_group_name.set(String::new());
                                            selected_peers_for_group.set(std::collections::HashSet::new());
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

    let bubble_class = if is_me { "message-bubble message-bubble-me" } else { "message-bubble message-bubble-them" };

    rsx! {
        div {
            class: if is_me { "message-row message-row-me" } else { "message-row message-row-them" },
            div {
                class: "{bubble_class}",
                if let Some((cid, key, nonce, mime, filename)) = file_data {
                    FileAttachment { cid, encryption_key: key, nonce, mime, filename }
                } else {
                    span { class: "message-text", "{content}" }
                }
                div { class: "message-time",
                    "{msg.timestamp}"
                    if is_me {
                         span { class: "message-status", " ✓✓" }
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
        div { class: "file-attachment",
            div { class: "file-icon", "📄" }
            div { class: "file-info",
                div { class: "file-name", "{filename}" }
                div { class: "file-type", "Encrypted File" }
            }
            if let Some(url) = decrypted_url.read().clone() {
                a { 
                    class: "file-download-btn",
                    href: "{url}",
                    download: "{filename_download}",
                    "⬇️"
                }
            } else if *is_decrypting.read() {
                span { class: "file-loading", "⏳" }
            }
        }
        if let Some(url) = decrypted_url.read().clone() {
             if mime_image_render.starts_with("image/") {
                 img { src: "{url}", class: "file-image-preview" }
             }
        }
    }
}

const MESSAGING_CSS: &str = r#"
/* Messaging Page Styles */
.messaging-container {
    display: flex;
    height: 100vh;
    background: var(--bg-void);
    overflow: hidden;
}

/* Sidebar */
.messaging-sidebar {
    width: 320px;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-right: 1px solid var(--border-default);
}

.sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-default);
}

.sidebar-title {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--text-primary);
}

.sidebar-actions {
    display: flex;
    gap: 0.5rem;
}

.icon-btn {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 50%;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.2s ease;
}

.icon-btn:hover {
    background: var(--primary);
    border-color: var(--primary);
    transform: scale(1.05);
}

.sidebar-search {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-subtle);
}

.search-input {
    width: 100%;
    padding: 0.625rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 20px;
    color: var(--text-primary);
    font-size: 0.875rem;
    outline: none;
    transition: all 0.2s ease;
}

.search-input:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(0, 229, 255, 0.1);
}

.search-input::placeholder {
    color: var(--text-muted);
}

/* Chat List */
.chat-list {
    flex: 1;
    overflow-y: auto;
}

.chat-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.875rem 1rem;
    cursor: pointer;
    transition: background 0.15s ease;
    border-left: 3px solid transparent;
}

.chat-item:hover {
    background: var(--bg-elevated);
}

.chat-item-active {
    background: var(--bg-elevated);
    border-left-color: var(--primary);
}

.chat-avatar {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--bg-deep);
    border: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    flex-shrink: 0;
}

.chat-avatar-group {
    background: var(--secondary);
    color: white;
    border: none;
}

.chat-info {
    flex: 1;
    min-width: 0;
}

.chat-name {
    font-weight: 600;
    font-size: 0.9375rem;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.chat-preview {
    font-size: 0.8125rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.empty-chats {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    text-align: center;
}

.empty-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
    opacity: 0.5;
}

.empty-text {
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
}

.empty-hint {
    font-size: 0.8125rem;
    color: var(--text-muted);
}

/* Chat Area */
.chat-area {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-deep);
}

.chat-empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 2rem;
}

.empty-state-icon {
    font-size: 4rem;
    margin-bottom: 1.5rem;
    opacity: 0.6;
}

.empty-state-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-bottom: 0.5rem;
}

.empty-state-text {
    color: var(--text-muted);
    margin-bottom: 1.5rem;
}

.chat-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.875rem 1.25rem;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-default);
}

.chat-header-info {
    display: flex;
    align-items: center;
    gap: 0.75rem;
}

.chat-header-avatar {
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: var(--bg-elevated);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    color: var(--text-secondary);
}

.chat-header-name {
    font-weight: 600;
    color: var(--text-primary);
}

.chat-header-status {
    font-size: 0.75rem;
    color: var(--text-muted);
}

/* Messages */
.messages-area {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column-reverse;
}

.messages-container {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.message-row {
    display: flex;
    margin-bottom: 0.25rem;
}

.message-row-me {
    justify-content: flex-end;
}

.message-row-them {
    justify-content: flex-start;
}

.message-bubble {
    max-width: 70%;
    padding: 0.625rem 0.875rem;
    border-radius: 16px;
    position: relative;
}

.message-bubble-me {
    background: var(--primary);
    color: var(--bg-void);
    border-bottom-right-radius: 4px;
}

.message-bubble-them {
    background: var(--bg-surface);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-bottom-left-radius: 4px;
}

.message-text {
    font-size: 0.9375rem;
    line-height: 1.4;
    word-wrap: break-word;
}

.message-time {
    font-size: 0.6875rem;
    opacity: 0.7;
    margin-top: 0.25rem;
    text-align: right;
}

.message-status {
    font-weight: 600;
}

/* Input Area */
.message-input-area {
    padding: 0.75rem 1rem;
    background: var(--bg-surface);
    border-top: 1px solid var(--border-default);
}

.input-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.file-upload-btn {
    position: relative;
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border-radius: 50%;
    cursor: pointer;
    font-size: 1.125rem;
}

.file-upload-btn:hover {
    background: var(--bg-deep);
}

.file-input-hidden {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
}

.message-input {
    flex: 1;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 24px;
    color: var(--text-primary);
    font-size: 0.9375rem;
    outline: none;
    transition: all 0.2s ease;
}

.message-input:focus {
    border-color: var(--primary);
}

.message-input::placeholder {
    color: var(--text-muted);
}

.send-btn {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border: none;
    border-radius: 50%;
    cursor: pointer;
    font-size: 1.25rem;
    color: var(--text-muted);
    transition: all 0.2s ease;
}

.send-btn-active {
    background: var(--primary);
    color: var(--bg-void);
}

.send-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

/* Modal Styles */
.modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
}

.modal-content {
    background: var(--bg-surface);
    border: 1px solid var(--border-default);
    border-radius: 16px;
    width: 90%;
    max-width: 420px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: slideUp 0.25s ease;
}

.modal-content-large {
    max-width: 500px;
    max-height: 85vh;
}

@keyframes slideUp {
    from { 
        opacity: 0;
        transform: translateY(20px);
    }
    to { 
        opacity: 1;
        transform: translateY(0);
    }
}

.modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border-default);
}

.modal-title {
    font-size: 1.125rem;
    font-weight: 700;
    color: var(--text-primary);
}

.modal-close-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 50%;
    cursor: pointer;
    font-size: 1rem;
    color: var(--text-secondary);
    transition: all 0.15s ease;
}

.modal-close-btn:hover {
    background: var(--error);
    border-color: var(--error);
    color: white;
}

.modal-body {
    padding: 1.25rem;
    overflow-y: auto;
    flex: 1;
}

.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    padding: 1rem 1.25rem;
    border-top: 1px solid var(--border-default);
    background: var(--bg-elevated);
}

/* Form Elements */
.input-group {
    margin-bottom: 1.25rem;
}

.input-label {
    display: block;
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
}

.modal-input {
    width: 100%;
    padding: 0.75rem 1rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    color: var(--text-primary);
    font-size: 0.9375rem;
    outline: none;
    transition: all 0.2s ease;
}

.modal-input:focus {
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(0, 229, 255, 0.1);
}

.modal-input::placeholder {
    color: var(--text-muted);
}

.input-hint {
    font-size: 0.75rem;
    color: var(--text-muted);
    margin-top: 0.375rem;
}

.add-member-row {
    display: flex;
    gap: 0.5rem;
}

.add-member-row .modal-input {
    flex: 1;
}

/* Buttons */
.btn-primary {
    padding: 0.75rem 1.5rem;
    background: var(--gradient-primary);
    border: none;
    border-radius: 8px;
    color: var(--bg-void);
    font-weight: 600;
    font-size: 0.9375rem;
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-primary:hover:not(:disabled) {
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0, 229, 255, 0.3);
}

.btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.btn-full {
    width: 100%;
}

.btn-secondary {
    padding: 0.75rem 1.5rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: 8px;
    color: var(--text-primary);
    font-weight: 600;
    font-size: 0.9375rem;
    cursor: pointer;
    transition: all 0.2s ease;
}

.btn-secondary:hover {
    background: var(--bg-deep);
}

/* Section Divider */
.section-divider {
    display: flex;
    align-items: center;
    text-align: center;
    margin: 1.5rem 0;
    color: var(--text-muted);
    font-size: 0.75rem;
}

.section-divider::before,
.section-divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--border-subtle);
}

.section-divider span {
    padding: 0 0.75rem;
}

/* Peer List */
.peer-list {
    max-height: 200px;
    overflow-y: auto;
}

.peer-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.625rem 0.75rem;
    cursor: pointer;
    border-radius: 8px;
    transition: background 0.15s ease;
}

.peer-item:hover {
    background: var(--bg-elevated);
}

.peer-avatar {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--bg-deep);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: 600;
    color: var(--text-secondary);
    font-size: 0.875rem;
}

.peer-info {
    flex: 1;
    min-width: 0;
}

.peer-name {
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.peer-label {
    font-size: 0.75rem;
    color: var(--text-muted);
}

.add-indicator {
    font-size: 1.25rem;
    color: var(--primary);
    font-weight: 700;
}

/* Selected Members */
.selected-members {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.selected-member-chip {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.375rem 0.75rem;
    background: var(--bg-elevated);
    border: 1px solid var(--border-default);
    border-radius: 20px;
    font-size: 0.8125rem;
    color: var(--text-primary);
}

.chip-remove {
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 50%;
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: all 0.15s ease;
}

.chip-remove:hover {
    background: var(--error);
    color: white;
}

/* File Attachment */
.file-attachment {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem;
    background: rgba(0, 0, 0, 0.1);
    border-radius: 8px;
}

.file-icon {
    font-size: 1.5rem;
}

.file-info {
    flex: 1;
    min-width: 0;
}

.file-name {
    font-weight: 600;
    font-size: 0.875rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.file-type {
    font-size: 0.75rem;
    opacity: 0.7;
}

.file-download-btn {
    padding: 0.375rem;
    background: rgba(255, 255, 255, 0.1);
    border-radius: 4px;
    text-decoration: none;
}

.file-image-preview {
    margin-top: 0.5rem;
    max-width: 100%;
    max-height: 200px;
    border-radius: 8px;
    object-fit: contain;
}

/* Scrollbar */
.chat-list::-webkit-scrollbar,
.messages-area::-webkit-scrollbar,
.modal-body::-webkit-scrollbar,
.peer-list::-webkit-scrollbar {
    width: 6px;
}

.chat-list::-webkit-scrollbar-track,
.messages-area::-webkit-scrollbar-track,
.modal-body::-webkit-scrollbar-track,
.peer-list::-webkit-scrollbar-track {
    background: transparent;
}

.chat-list::-webkit-scrollbar-thumb,
.messages-area::-webkit-scrollbar-thumb,
.modal-body::-webkit-scrollbar-thumb,
.peer-list::-webkit-scrollbar-thumb {
    background: var(--border-default);
    border-radius: 3px;
}

.chat-list::-webkit-scrollbar-thumb:hover,
.messages-area::-webkit-scrollbar-thumb:hover,
.modal-body::-webkit-scrollbar-thumb:hover,
.peer-list::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
}
"#;