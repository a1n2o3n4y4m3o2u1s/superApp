use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::components::common::BlobImage;

#[component]
pub fn ProfileComponent(peer_id: Option<String>) -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let local_id = app_state.local_peer_id.read().clone();
    let is_own_profile = peer_id.is_none() || peer_id.as_ref() == Some(&local_id);
    let target_id = peer_id.clone().unwrap_or(local_id.clone());

    // Form signals
    let mut name = use_signal(|| "".to_string());
    let mut bio = use_signal(|| "".to_string());
    let mut recipient_id = use_signal(|| "".to_string());
    let mut amount = use_signal(|| "".to_string());
    

    // Tabs
    let mut active_tab = use_signal(|| "info".to_string());
    let user_posts = app_state.user_posts;

    // File Upload State
    let mut upload_filename = use_signal(|| "".to_string());
    let mut upload_mime = use_signal(|| "".to_string());
    let mut upload_data = use_signal(|| Vec::<u8>::new());

    // Setup file reader eval
    // Setup file reader eval
    
    use_effect(move || {
        // Only setup if we haven't yet (or if we need to re-attach, but id is constant)
        // We use a slight delay or just run it. Dioxus eval runs immediately.
        // We attach listener to ID 'file-upload-input'
        
        let mut eval = document::eval(r#"
            // Wait for element to exist roughly
            setTimeout(() => {
                const input = document.getElementById('file-upload-input');
                if (input) {
                    input.addEventListener('change', (e) => {
                        const file = e.target.files[0];
                        if (!file) return;
                        
                        const reader = new FileReader();
                        reader.onload = (evt) => {
                             const dataUrl = evt.target.result;
                             const b64 = dataUrl.split(',')[1];
                             dioxus.send({ name: file.name, mime: file.type, data: b64 });
                        };
                        reader.readAsDataURL(file);
                    });
                }
            }, 500); // Small delay to ensure render
        "#);

        spawn(async move {
            while let Ok(msg) = eval.recv().await {
                if let Ok(obj) = serde_json::from_value::<serde_json::Value>(msg) {
                    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                        upload_filename.set(name.to_string());
                    }
                    if let Some(mime) = obj.get("mime").and_then(|v| v.as_str()) {
                         // Default to application/octet-stream if empty
                         if mime.is_empty() {
                             upload_mime.set("application/octet-stream".to_string());
                         } else {
                             upload_mime.set(mime.to_string());
                         }
                    }
                    if let Some(data_b64) = obj.get("data").and_then(|v| v.as_str()) {
                        use base64::{Engine as _, engine::general_purpose::STANDARD};
                        if let Ok(bytes) = STANDARD.decode(data_b64) {
                            upload_data.set(bytes);
                        }
                    }
                }
            }
        });
    });

    // Initial data fetch - runs once on mount
    let mut has_fetched = use_signal(|| false);
    let cmd_tx_fetch = cmd_tx.clone();
    let target_id_fetch = target_id.clone();
    if !has_fetched() {
        has_fetched.set(true);
        if is_own_profile {
            let _ = cmd_tx_fetch.send(AppCmd::FetchMyProfile);
            let _ = cmd_tx_fetch.send(AppCmd::FetchBalance);
            let _ = cmd_tx_fetch.send(AppCmd::FetchPendingTransfers);
            let _ = cmd_tx_fetch.send(AppCmd::FetchUbiTimer);
            let _ = cmd_tx_fetch.send(AppCmd::FetchStorageStats);
            let _ = cmd_tx_fetch.send(AppCmd::FetchMyWebPages);
            let _ = cmd_tx_fetch.send(AppCmd::FetchMyFiles);
            let _ = cmd_tx_fetch.send(AppCmd::FetchReputation { peer_id: target_id_fetch.clone() });
            let _ = cmd_tx_fetch.send(AppCmd::FetchMyCertifications);
        } else {
            let _ = cmd_tx_fetch.send(AppCmd::FetchUserProfile { peer_id: target_id_fetch.clone() });
            let _ = cmd_tx_fetch.send(AppCmd::FetchReputation { peer_id: target_id_fetch.clone() });
            let _ = cmd_tx_fetch.send(AppCmd::FetchCertifications { peer_id: target_id_fetch.clone() });
        }
        // Always fetch posts for the profile we are viewing
        let _ = cmd_tx_fetch.send(AppCmd::FetchGivenUserPosts { peer_id: target_id_fetch.clone() });
    }

    // UBI Timer - compute once before RSX
    let ubi_timer = *app_state.ubi_timer.read();
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let (time_remaining, can_claim) = if let Some(last_claim) = ubi_timer {
        let next_claim = last_claim + 86400;
        if current_time >= next_claim {
            ("Ready!".to_string(), true)
        } else {
            let diff = next_claim - current_time;
            let hours = diff / 3600;
            let minutes = (diff % 3600) / 60;
            let seconds = diff % 60;
            (format!("{:02}:{:02}:{:02}", hours, minutes, seconds), false)
        }
    } else {
        ("Ready!".to_string(), true)
    };

    // Sync profile data to form once
    let mut profile_synced = use_signal(|| false);
    if is_own_profile && !profile_synced() {
        if let Some(p) = app_state.profile.read().as_ref() {
            name.set(p.name.clone());
            bio.set(p.bio.clone());
            profile_synced.set(true);
        }
    }

    // Pre-read all state BEFORE RSX to avoid reactive issues
    let balance = *app_state.balance.read();
    let pending_transfers = app_state.pending_transfers.read().clone();
    let reputation = app_state.reputation.read().clone();
    let contracts = app_state.contracts.read().clone();
    let contract_states = app_state.contract_states.read().clone();
    let web_pages = app_state.my_web_pages.read().clone();
    let my_files = app_state.files.read().clone();
    let verification_status = app_state.verification_status.read().clone();
    let following = app_state.following.read();
    let is_following = following.contains(&target_id);
    
    let display_profile = if is_own_profile {
        app_state.profile.read().clone()
    } else {
        app_state.viewed_profile.read().clone()
    };

    let display_name = display_profile.as_ref().map(|p| p.name.clone()).unwrap_or("Unknown".to_string());
    let display_bio = display_profile.as_ref().map(|p| p.bio.clone()).unwrap_or("No bio available".to_string());
    let founder_id = display_profile.as_ref().and_then(|p| p.founder_id);
    let is_verified_viewer = matches!(verification_status, crate::backend::VerificationStatus::Verified | crate::backend::VerificationStatus::Founder);

    // Event handlers
    let cmd_tx_submit = cmd_tx.clone();
    let on_submit = move |_| {
        let _ = cmd_tx_submit.send(AppCmd::PublishProfile { name: name(), bio: bio(), photo: None });
        let _ = cmd_tx_submit.send(AppCmd::FetchMyProfile);
    };

    let cmd_tx_mint = cmd_tx.clone();
    let on_mint = move |_| { let _ = cmd_tx_mint.send(AppCmd::ClaimUbi); };

    let cmd_tx_send = cmd_tx.clone();
    let on_send = move |_| {
        if let Ok(amt) = amount().parse::<u64>() {
            let _ = cmd_tx_send.send(AppCmd::SendToken { recipient: recipient_id(), amount: amt });
        }
    };

    let cmd_tx_vouch = cmd_tx.clone();
    let target_id_vouch = target_id.clone();
    let on_vouch = move |_| {
        let _ = cmd_tx_vouch.send(AppCmd::Vouch { target_peer_id: target_id_vouch.clone() });
    };

    let cmd_tx_follow = cmd_tx.clone();
    let target_id_follow = target_id.clone();
    let on_follow = move |_| {
        let _ = cmd_tx_follow.send(AppCmd::FollowUser { target: target_id_follow.clone(), follow: !is_following });
    };





    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Header
            div { class: "page-header",
                div { class: "flex justify-between items-start",
                    div {
                        h1 { class: "page-title", 
                            if is_own_profile { "Profile" } else { "User Profile" } 
                        }
                        p { class: "text-[var(--text-secondary)]", 
                            if is_own_profile { "Manage your identity" } else { "View user details" }
                        }
                    }
                    div { class: "flex items-center gap-2",
                        if !is_own_profile {
                             button { 
                                 class: if is_following { "btn btn-secondary" } else { "btn btn-primary" },
                                 onclick: on_follow,
                                 if is_following { "Unfollow" } else { "Follow" }
                             }
                        }
                        if !is_own_profile && is_verified_viewer {
                            button { class: "btn btn-primary", onclick: on_vouch, "✓ Vouch" }
                        }
                    }
                }
            }

            // Tabs
            div { class: "flex gap-2 mb-6 border-b border-[var(--border-subtle)] pb-2",
                div { 
                    class: if active_tab() == "info" { "nav-button active cursor-pointer" } else { "nav-button cursor-pointer" },
                    onclick: move |_| active_tab.set("info".to_string()),
                    "Info"
                }
                div { 
                    class: if active_tab() == "posts" { "nav-button active cursor-pointer" } else { "nav-button cursor-pointer" },
                    onclick: move |_| active_tab.set("posts".to_string()),
                    "Posts"
                }
            }

            if active_tab() == "posts" {
                // Posts Grid
                if user_posts().is_empty() {
                    div { class: "empty-state py-12",
                        p { class: "text-2xl mb-2", "📷" }
                        p { class: "empty-state-text", "No posts yet" }
                    }
                } else {
                    div { class: "grid grid-cols-3 gap-1 md:gap-4",
                        for node in user_posts().iter() {
                             if let crate::backend::dag::DagPayload::Post(ref p) = node.payload {
                                 {
                                     let content = p.content.clone();
                                     let has_img = !p.attachments.is_empty();
                                     let first_img = p.attachments.first().cloned();
                                     
                                     rsx! {
                                         div { class: "aspect-square bg-[var(--bg-elevated)] relative overflow-hidden group cursor-pointer border border-[var(--border-color)] rounded-lg",
                                             if let Some(cid) = first_img {
                                                   BlobImage { cid: cid.clone(), class: Some("w-full h-full object-cover transition-transform duration-500 group-hover:scale-110".to_string()) }
                                                  div { class: "absolute inset-0 bg-black/0 group-hover:bg-black/20 transition-colors duration-300" }
                                             } else {
                                                 div { class: "w-full h-full flex items-center justify-center p-4 text-center text-xs text-[var(--text-muted)] group-hover:text-[var(--text-primary)] transition-colors", "{content}" }
                                             }
                                         }
                                     }
                                 }
                             }
                        }
                    }
                }
            } else {
            // Grid
            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                
                // Left column
                div { class: "section-stack",
                    
                    // Identity card
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "Identity" }
                            if let Some(fid) = founder_id {
                                span { class: "badge badge-founder", "Founder #{fid}" }
                            }
                        }
                        div { class: "card mb-4",
                            p { class: "font-mono text-sm break-all text-[var(--text-primary)]", "{target_id}" }
                        }
                        
                        // Certifications Badge Display
                        {
                            let certs = app_state.certifications.read();
                            if !certs.is_empty() {
                                rsx! {
                                    div { class: "border-t border-[var(--border-subtle)] pt-4 mt-2",
                                        p { class: "text-sm font-bold mb-2 flex items-center gap-2", "🏆 Certifications" }
                                        div { class: "flex flex-wrap gap-2",
                                            for node in certs.iter() {
                                                if let crate::backend::dag::DagPayload::Certification(c) = &node.payload {
                                                    {
                                                        let type_str = format!("{:?}", c.certification_type);
                                                        // Pretty print enum variants
                                                        let display_str = match type_str.as_str() {
                                                            "CivicLiteracy" => "Civic Literacy",
                                                            "GovernanceRoles" => "Governance Roles",
                                                            "TechnicalSkills" => "Technical Skills",
                                                            "TradeQualifications" => "Trade Qualifications",
                                                            "ModerationJury" => "Jury Qualified",
                                                            _ => &type_str,
                                                        };
                                                        let icon = match type_str.as_str() {
                                                            "CivicLiteracy" => "📜",
                                                            "GovernanceRoles" => "⚖️",
                                                            "TechnicalSkills" => "💻",
                                                            "TradeQualifications" => "🛠️",
                                                            "ModerationJury" => "🧑‍⚖️",
                                                            _ => "🏅",
                                                        };
                                                        
                                                        rsx! {
                                                            div { class: "px-2 py-1 rounded bg-[var(--bg-secondary)] border border-[var(--border-color)] text-xs flex items-center gap-1",
                                                                span { "{icon}" }
                                                                span { "{display_str}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                rsx!({})
                            }
                        }
                    }

                    // Reputation - using pre-read value
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "Reputation" }
                            if let Some(ref rep) = reputation {
                                {
                                    let score = rep.score;
                                    rsx! { span { class: "badge badge-primary", "Score: {score}" } }
                                }
                            }
                        }
                        if let Some(ref rep) = reputation {
                            {
                                let verif = rep.breakdown.verification;
                                let cont = rep.breakdown.content;
                                let gov = rep.breakdown.governance;
                                let stor = rep.breakdown.storage;
                                rsx! {
                                    div { class: "grid grid-cols-2 gap-4",
                                        div { class: "card text-center p-2",
                                            p { class: "text-xs text-[var(--text-secondary)]", "Verification" }
                                            p { class: "text-lg font-bold", "{verif}" }
                                        }
                                        div { class: "card text-center p-2",
                                            p { class: "text-xs text-[var(--text-secondary)]", "Content" }
                                            p { class: "text-lg font-bold", "{cont}" }
                                        }
                                        div { class: "card text-center p-2",
                                            p { class: "text-xs text-[var(--text-secondary)]", "Governance" }
                                            p { class: "text-lg font-bold", "{gov}" }
                                        }
                                        div { class: "card text-center p-2",
                                            p { class: "text-xs text-[var(--text-secondary)]", "Storage" }
                                            p { class: "text-lg font-bold", "{stor}" }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "empty-state py-4", "Loading reputation..." }
                        }
                    }

                    // Profile form / view
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", if is_own_profile { "Edit Profile" } else { "About" } }
                        }
                        if is_own_profile {
                            div { class: "form-group",
                                label { class: "form-label", "Display Name" }
                                input {
                                    class: "input",
                                    placeholder: "Your name",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Bio" }
                                textarea {
                                    class: "input",
                                    style: "min-height: 100px; resize: none;",
                                    placeholder: "Tell us about yourself...",
                                    value: "{bio}",
                                    oninput: move |e| bio.set(e.value())
                                }
                            }
                            div { class: "action-group",
                                button { class: "btn btn-primary", onclick: on_submit, "Save Changes" }
                            }
                        } else {
                            div { class: "space-y-4",
                                div {
                                    p { class: "label mb-1", "Name" }
                                    p { class: "text-lg font-medium", "{display_name}" }
                                }
                                div {
                                    p { class: "label mb-1", "Bio" }
                                    p { class: "text-[var(--text-secondary)] whitespace-pre-wrap", "{display_bio}" }
                                }
                            }
                        }
                    }
                }

                // Right column (Wallet & more) - only for own profile
                if is_own_profile {
                    div { class: "section-stack",
                        
                        // Wallet
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "Wallet" }
                            }
                            
                            // Balance
                            div { class: "card mb-4",
                                div { class: "flex justify-between items-end",
                                    div {
                                        p { class: "label", "Balance" }
                                        p { class: "text-3xl font-bold mt-1", "{balance} SUPER" }
                                    }
                                    button {
                                        class: if can_claim { "btn btn-secondary btn-sm" } else { "btn btn-secondary btn-sm opacity-50" },
                                        onclick: on_mint,
                                        disabled: !can_claim,
                                        "Claim UBI"
                                    }
                                }
                                p { class: "text-xs text-[var(--text-muted)] mt-2", "Next claim: {time_remaining}" }
                            }

                            // Send
                            div { class: "divider" }
                            p { class: "font-medium mb-4", "Send Tokens" }
                            div { class: "form-group",
                                label { class: "form-label", "Recipient" }
                                input {
                                    class: "input",
                                    placeholder: "Peer ID",
                                    value: "{recipient_id}",
                                    oninput: move |e| recipient_id.set(e.value())
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Amount" }
                                input {
                                    class: "input",
                                    r#type: "number",
                                    placeholder: "0",
                                    value: "{amount}",
                                    oninput: move |e| amount.set(e.value())
                                }
                            }
                            button { class: "btn btn-primary", onclick: on_send, "Send" }

                            // Pending transfers
                            if !pending_transfers.is_empty() {
                                div { class: "divider" }
                                p { class: "font-medium mb-2", "Incoming Transfers" }
                                for node in pending_transfers.iter() {
                                    if let crate::backend::dag::DagPayload::Token(ref t) = node.payload {
                                        {
                                            let amt = t.amount;
                                            let sender = node.author.clone();
                                            let burn_cid = node.id.clone();
                                            let cmd_tx_claim = cmd_tx.clone();
                                            rsx! {
                                                div { class: "list-item flex justify-between items-center",
                                                    div { class: "list-item-content",
                                                        p { class: "list-item-title", "{amt} SUPER" }
                                                        p { class: "list-item-subtitle truncate", "From: {sender.get(0..12).unwrap_or(&sender)}..." }
                                                    }
                                                    button { 
                                                        class: "btn btn-success btn-sm",
                                                        onclick: move |_| { 
                                                            let _ = cmd_tx_claim.send(AppCmd::ClaimToken { burn_cid: burn_cid.clone() }); 
                                                        },
                                                        "Claim"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Storage Stats - Device storage info
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "📊 Device Storage" }
                                {
                                    let cmd_tx_storage = cmd_tx.clone();
                                    rsx! {
                                        button { 
                                            class: "btn btn-sm btn-secondary",
                                            onclick: move |_| {
                                                let _ = cmd_tx_storage.send(AppCmd::FetchStorageStats);
                                            },
                                            "Refresh"
                                        }
                                    }
                                }
                            }
                            {
                                let stats = app_state.storage_stats.read();
                                rsx! {
                                    div { class: "grid grid-cols-2 gap-4",
                                        div { class: "card text-center",
                                            p { class: "text-3xl font-bold text-[var(--primary)]", "{stats.0}" }
                                            p { class: "text-sm text-[var(--text-secondary)]", "Total Nodes" }
                                        }
                                        div { class: "card text-center",
                                            {
                                                let size_mb = stats.1 as f64 / 1024.0 / 1024.0;
                                                let size_display = if size_mb >= 1.0 {
                                                    format!("{:.2} MB", size_mb)
                                                } else {
                                                    format!("{:.2} KB", stats.1 as f64 / 1024.0)
                                                };
                                                rsx! {
                                                    p { class: "text-3xl font-bold text-[var(--accent)]", "{size_display}" }
                                                    p { class: "text-sm text-[var(--text-secondary)]", "Storage Used" }
                                                }
                                            }
                                        }
                                    }
                                    
                                    // Storage Quota Setting
                                    div { class: "mt-4 pt-4 border-t border-[var(--border-subtle)]",
                                        p { class: "text-sm font-medium text-[var(--text-primary)] mb-2", "Storage Quota (optional)" }
                                        div { class: "flex gap-2 flex-wrap",
                                            {
                                                let cmd_tx_q1 = cmd_tx.clone();
                                                let cmd_tx_q2 = cmd_tx.clone();
                                                let cmd_tx_q3 = cmd_tx.clone();
                                                let cmd_tx_q4 = cmd_tx.clone();
                                                rsx! {
                                                    button {
                                                        class: "btn btn-sm btn-secondary",
                                                        onclick: move |_| { let _ = cmd_tx_q1.send(AppCmd::SetStorageQuota { quota_mb: None }); },
                                                        "Unlimited"
                                                    }
                                                    button {
                                                        class: "btn btn-sm btn-secondary",
                                                        onclick: move |_| { let _ = cmd_tx_q2.send(AppCmd::SetStorageQuota { quota_mb: Some(500) }); },
                                                        "500 MB"
                                                    }
                                                    button {
                                                        class: "btn btn-sm btn-secondary",
                                                        onclick: move |_| { let _ = cmd_tx_q3.send(AppCmd::SetStorageQuota { quota_mb: Some(1024) }); },
                                                        "1 GB"
                                                    }
                                                    button {
                                                        class: "btn btn-sm btn-secondary",
                                                        onclick: move |_| { let _ = cmd_tx_q4.send(AppCmd::SetStorageQuota { quota_mb: Some(5120) }); },
                                                        "5 GB"
                                                    }
                                                }
                                            }
                                        }
                                        p { class: "text-xs text-[var(--text-muted)] mt-2",
                                            "Default: Unlimited. Quota only affects local storage, not network replication."
                                        }
                                    }
                                    
                                    p { class: "text-xs text-[var(--text-muted)] mt-4 text-center",
                                        "DAG storage on this device. Data is replicated across network peers."
                                    }
                                }
                            }
                        }


                        // SuperWeb Pages
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "SuperWeb Pages" }
                            }
                            if web_pages.is_empty() {
                                div { class: "empty-state py-4",
                                    p { class: "empty-state-text", "No pages published yet" }
                                }
                            } else {
                                for node in web_pages.iter() {
                                    if let crate::backend::dag::DagPayload::Web(ref w) = node.payload {
                                        {
                                            let page_url = w.url.clone();
                                            let full_url = format!("sp://{}", w.url);
                                            rsx! {
                                                div { class: "list-item",
                                                    div { class: "list-item-content",
                                                        p { class: "list-item-title", "{page_url}" }
                                                        p { class: "list-item-subtitle", "{full_url}" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // My Files
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "My Files" }
                            }
                            
                            // Upload Section
                            div { class: "card mb-4 bg-[var(--bg-elevated)]",
                                p { class: "font-medium mb-2", "Upload File" }
                                input {
                                    id: "file-upload-input",
                                    r#type: "file",
                                    class: "file-input file-input-bordered w-full max-w-xs",
                                }
                                if !upload_filename().is_empty() {
                                    div { class: "mt-3 p-3 bg-[var(--bg-base)] rounded-lg",
                                        p { class: "text-sm font-medium", "Selected: {upload_filename}" }
                                        p { class: "text-xs text-[var(--text-muted)] mb-2", "Size: {upload_data().len()} bytes, Type: {upload_mime}" }
                                        button {
                                            class: "btn btn-primary btn-sm w-full",
                                            onclick: move |_| {
                                                if !upload_data().is_empty() {
                                                    let _ = cmd_tx.send(AppCmd::UploadFile {
                                                        name: upload_filename(),
                                                        mime_type: upload_mime(),
                                                        data: upload_data()
                                                    });
                                                    // Reset locally
                                                    upload_filename.set("".to_string());
                                                    upload_data.set(vec![]);
                                                    
                                                    // Clear input
                                                    let mut eval = document::eval("document.getElementById('file-upload-input').value = '';");
                                                    spawn(async move { let _ = eval.recv::<serde_json::Value>().await; });
                                                }
                                            },
                                            "Upload to Network"
                                        }
                                    }
                                }
                            }
                            if my_files.is_empty() {
                                div { class: "empty-state py-4",
                                    p { class: "empty-state-text", "No files uploaded yet" }
                                }
                            } else {
                                for node in my_files.iter() {
                                    if let crate::backend::dag::DagPayload::File(ref f) = node.payload {
                                        {
                                            let fname = f.name.clone();
                                            let fmime = f.mime_type.clone();
                                            let fsize = if f.size > 1024 * 1024 {
                                                format!("{:.2} MB", f.size as f64 / 1024.0 / 1024.0)
                                            } else {
                                                format!("{} KB", f.size / 1024)
                                            };
                                            let blob_cid = f.blob_cid.clone();
                                            
                                            rsx! {
                                                div { class: "list-item flex justify-between items-center group",
                                                    div { class: "list-item-content",
                                                        p { class: "list-item-title", "{fname}" }
                                                        p { class: "list-item-subtitle", "{fmime} • {fsize}" }
                                                        p { class: "text-xs font-mono text-[var(--text-muted)] truncate", "CID: {blob_cid}" }
                                                    }
                                                    button {
                                                        class: "btn btn-ghost btn-sm opacity-0 group-hover:opacity-100 transition-opacity",
                                                        onclick: move |_| {
                                                            let mut eval = document::eval(&format!("navigator.clipboard.writeText('{}')", blob_cid));
                                                            spawn(async move { let _ = eval.recv::<serde_json::Value>().await; });
                                                        },
                                                        "Copy CID"
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
        }
    }
}

#[component]
pub fn UserProfileComponent(peer_id: String) -> Element {
    rsx! {
        ProfileComponent { peer_id: Some(peer_id) }
    }
}
