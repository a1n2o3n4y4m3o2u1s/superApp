use dioxus::prelude::*;
use crate::backend::AppCmd;

#[component]
pub fn ProfileComponent(peer_id: Option<String>) -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let local_id = app_state.local_peer_id.read().clone();
    let is_own_profile = peer_id.is_none() || peer_id.as_ref() == Some(&local_id);
    let target_id = peer_id.clone().unwrap_or(local_id.clone());

    let mut name = use_signal(|| "".to_string());
    let mut bio = use_signal(|| "".to_string());
    let mut recipient_id = use_signal(|| "".to_string());

    let mut amount = use_signal(|| "".to_string());
    
    // Contract State
    let mut show_new_contract = use_signal(|| false);
    let mut contract_code = use_signal(|| "".to_string());
    let mut contract_init_params = use_signal(|| "".to_string());
    
    let mut active_call_contract = use_signal(|| None::<String>);
    let mut call_method = use_signal(|| "".to_string());
    let mut call_params = use_signal(|| "".to_string());

    let cmd_tx_clone = cmd_tx.clone();
    let target_id_clone = target_id.clone();
    use_effect(move || {
        let tid = target_id_clone.clone();
        if is_own_profile {
            let _ = cmd_tx_clone.send(AppCmd::FetchMyProfile);
            let _ = cmd_tx_clone.send(AppCmd::FetchBalance);
            let _ = cmd_tx_clone.send(AppCmd::FetchPendingTransfers);

            let _ = cmd_tx_clone.send(AppCmd::FetchUbiTimer);
            let _ = cmd_tx_clone.send(AppCmd::FetchContracts);
        } else {
            let _ = cmd_tx_clone.send(AppCmd::FetchUserProfile { peer_id: tid });
        }
    });

    // UBI Timer
    let mut time_remaining = use_signal(|| "Loading...".to_string());
    let mut can_claim = use_signal(|| false);
    let mut now_seconds = use_signal(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    
    use_future(move || async move {
        loop {
            #[cfg(not(target_arch = "wasm32"))]
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            #[cfg(target_arch = "wasm32")]
            gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
            now_seconds.set(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
        }
    });

    use_effect(move || {
        if is_own_profile {
            let current_time = now_seconds();
            let timer = app_state.ubi_timer.read();
            if let Some(last_claim) = *timer {
                let next_claim = last_claim + 86400;
                if current_time >= next_claim {
                    time_remaining.set("Ready!".to_string());
                    can_claim.set(true);
                } else {
                    let diff = next_claim - current_time;
                    let hours = diff / 3600;
                    let minutes = (diff % 3600) / 60;
                    let seconds = diff % 60;
                    time_remaining.set(format!("{:02}:{:02}:{:02}", hours, minutes, seconds));
                    can_claim.set(false);
                }
            } else {
                time_remaining.set("Ready!".to_string());
                can_claim.set(true);
            }
        }
    });

    use_effect(move || {
        if is_own_profile {
            if let Some(p) = app_state.profile.read().as_ref() {
                name.set(p.name.clone());
                bio.set(p.bio.clone());
            }
        }
    });

    let cmd_tx_submit = cmd_tx.clone();
    let on_submit = move |_| {
        let cmd = AppCmd::PublishProfile { name: name(), bio: bio() };
        let _ = cmd_tx_submit.send(cmd);
        let _ = cmd_tx_submit.send(AppCmd::FetchMyProfile);
    };

    let cmd_tx_mint = cmd_tx.clone();
    let on_mint = move |_| { let _ = cmd_tx_mint.send(AppCmd::ClaimUbi); };

    let cmd_tx_send = cmd_tx.clone();
    let on_send = move |_| {
        if let Ok(amt) = amount().parse::<u64>() {
            let _ = cmd_tx_send.send(AppCmd::SendToken { recipient: recipient_id(), amount: amt });
            let cmd_tx_inner = cmd_tx_send.clone();
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(std::time::Duration::from_millis(500)).await;
                let _ = cmd_tx_inner.send(AppCmd::FetchBalance);
            });
        }
    };

    let cmd_tx_vouch = cmd_tx.clone();
    let target_id_vouch = target_id.clone();
    let on_vouch = move |_| {
        let _ = cmd_tx_vouch.send(AppCmd::Vouch { target_peer_id: target_id_vouch.clone() });
    };


    let cmd_tx_deploy = cmd_tx.clone();
    let on_deploy = move |_| {
         let _ = cmd_tx_deploy.send(AppCmd::DeployContract { 
             code: contract_code(), 
             init_params: contract_init_params() 
         });
         show_new_contract.set(false);
         // Reset form
         contract_code.set("".to_string());
         contract_init_params.set("".to_string());
    };

    let cmd_tx_call = cmd_tx.clone();
    let on_call = move |_| {
        if let Some(cid) = active_call_contract() {
            let _ = cmd_tx_call.send(AppCmd::CallContract {
                contract_id: cid,
                method: call_method(),
                params: call_params()
            });
            active_call_contract.set(None);
            call_method.set("".to_string());
            call_params.set("".to_string());
        }
    };

    let display_profile = if is_own_profile {
        app_state.profile.read().clone()
    } else {
        app_state.viewed_profile.read().clone()
    };

    let display_name = display_profile.as_ref().map(|p| p.name.clone()).unwrap_or("Unknown".to_string());
    let display_bio = display_profile.as_ref().map(|p| p.bio.clone()).unwrap_or("No bio available".to_string());
    let founder_id = display_profile.as_ref().and_then(|p| p.founder_id);
    let is_verified_viewer = matches!(*app_state.verification_status.read(), crate::backend::VerificationStatus::Verified | crate::backend::VerificationStatus::Founder);

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
                    if !is_own_profile && is_verified_viewer {
                        button { class: "btn btn-primary", onclick: on_vouch, "✓ Vouch" }
                    }
                }
            }

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
                        div { class: "card",
                            p { class: "label mb-2", "Peer ID" }
                            p { class: "font-mono text-sm break-all text-[var(--text-primary)]", "{target_id}" }
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

                // Right column (Wallet)
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
                                        p { class: "text-3xl font-bold mt-1", "{app_state.balance} SUPER" }
                                    }
                                    button {
                                        class: if can_claim() { "btn btn-secondary btn-sm" } else { "btn btn-secondary btn-sm opacity-50" },
                                        onclick: on_mint,
                                        disabled: !can_claim(),
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
                            button { class: "btn btn-primary w-full", onclick: on_send, "Send" }
                        }

                        // Pending
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "Pending Transfers" }
                            }
                            if app_state.pending_transfers.read().is_empty() {
                                div { class: "empty-state py-6",
                                    p { class: "empty-state-text", "No pending transfers" }
                                }
                            } else {
                                div { class: "space-y-2",
                                    {app_state.pending_transfers.read().iter().map(|node| {
                                        let node_id = node.id.clone();
                                        let node_author = node.author.clone();
                                        let amount_display = if let crate::backend::dag::DagPayload::Token(t) = &node.payload {
                                            format!("{} SUPER", t.amount)
                                        } else {
                                            "Unknown".to_string()
                                        };
                                        let cmd_tx_claim = cmd_tx.clone();

                                        rsx! {
                                            div {
                                                class: "list-item",
                                                key: "{node_id}",
                                                div { class: "list-item-content",
                                                    p { class: "list-item-title", "{amount_display}" }
                                                    p { class: "list-item-subtitle truncate", "From: {node_author}" }
                                                }
                                                button {
                                                    class: "btn btn-secondary btn-sm",
                                                    onclick: move |_| {
                                                        let _ = cmd_tx_claim.send(AppCmd::ClaimToken { burn_cid: node_id.clone() });
                                                    },
                                                    "Claim"
                                                }
                                            }
                                        }
                                    })}
                                }
                            }
                        }

                        // Smart Contracts
                        div { class: "panel",
                            div { class: "panel-header flex justify-between items-center",
                                h2 { class: "panel-title", "Smart Contracts" }
                                button { 
                                    class: "btn btn-secondary btn-sm", 
                                    onclick: move |_| show_new_contract.set(!show_new_contract()),
                                    if show_new_contract() { "Cancel" } else { "New" }
                                }
                            }
                            
                            if show_new_contract() {
                                div { class: "p-4 border-b border-[var(--border-default)]",
                                     div { class: "form-group",
                                         label { class: "form-label", "Code" }
                                         textarea { class: "input min-h-[80px]", value: "{contract_code}", oninput: move |e| contract_code.set(e.value()) }
                                     }
                                     div { class: "form-group",
                                         label { class: "form-label", "Init Params (JSON)" }
                                         input { class: "input", value: "{contract_init_params}", oninput: move |e| contract_init_params.set(e.value()) }
                                     }
                                     button { class: "btn btn-primary w-full", onclick: on_deploy, "Deploy" }
                                }
                            }

                            {
                                let contracts = app_state.contracts.read();
                                if contracts.is_empty() {
                                    rsx! {
                                        div { class: "empty-state py-6",
                                             p { class: "empty-state-text", "No contracts deployed" }
                                        }
                                    }
                                } else {
                                    rsx! {
                                        div { class: "space-y-2",
                                            for node in contracts.iter() {
                                                if let crate::backend::dag::DagPayload::Contract(contract) = &node.payload {
                                                    {
                                                        let cid = node.id.clone();
                                                        rsx! {
                                                            div { class: "list-item flex flex-col items-stretch",
                                                                div { class: "flex justify-between items-center w-full",
                                                                    div { class: "list-item-content",
                                                                        p { class: "list-item-title font-mono text-xs truncate", "{cid}" }
                                                                        p { class: "list-item-subtitle", "Params: {contract.init_params}" }
                                                                    }
                                                                    button { 
                                                                        class: "btn btn-secondary btn-sm", 
                                                                        onclick: {
                                                                            let cmd_tx_call_btn = cmd_tx.clone();
                                                                            let cid_call_btn = cid.clone();
                                                                            move |_| {
                                                                                if active_call_contract() == Some(cid_call_btn.clone()) {
                                                                                    active_call_contract.set(None);
                                                                                } else {
                                                                                    active_call_contract.set(Some(cid_call_btn.clone()));
                                                                                    let _ = cmd_tx_call_btn.send(AppCmd::FetchContractState { contract_id: cid_call_btn.clone() });
                                                                                }
                                                                            }
                                                                        },
                                                                        "Call" 
                                                                    }
                                                                }
                                                                // Trigger fetch state when opened is handled by effect or logic
                                                                {
                                                                    let cmd_tx_state = cmd_tx.clone();
                                                                    let cid_clone = cid.clone();
                                                                    let is_active = active_call_contract() == Some(cid.clone());
                                                                    
                                                                    // We can't use use_effect inside a loop easily without warnings, 
                                                                    // but we can just trigger it on click.
                                                                    // Let's move the onclick logic to a separate variable or closure above if needed, 
                                                                    // but actually we can just add it to the onclick handler.
                                                                }
                                                                if active_call_contract() == Some(cid.clone()) {
                                                                    div { class: "mt-2 pt-2 border-t border-[var(--border-default)]",
                                                                        // State Display
                                                                        div { class: "mb-4 p-3 bg-[var(--bg-secondary)] rounded-md font-mono text-xs overflow-x-auto",
                                                                            p { class: "text-[var(--text-muted)] mb-1", "Current State:" }
                                                                             {
                                                                                 let states = app_state.contract_states.read();
                                                                                 let state_json = states.get(&cid).cloned().unwrap_or("Loading...".to_string());
                                                                                 rsx! {
                                                                                     pre { "{state_json}" }
                                                                                 }
                                                                             }
                                                                        }

                                                                        div { class: "form-group",
                                                                            input { class: "input mb-2", placeholder: "Method (set/delete)", value: "{call_method}", oninput: move |e| call_method.set(e.value()) }
                                                                            input { class: "input mb-2", placeholder: "Params (JSON)", value: "{call_params}", oninput: move |e| call_params.set(e.value()) }
                                                                            button { class: "btn btn-primary w-full", onclick: on_call.clone(), "Execute Call" }
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

                        // Storage Stats
                        div { class: "panel",
                            div { class: "panel-header",
                                h2 { class: "panel-title", "Storage" }
                            }
                            {
                                let (block_count, total_bytes) = (app_state.storage_stats)();
                                let size_display = if total_bytes >= 1048576 {
                                    format!("{:.2} MB", total_bytes as f64 / 1048576.0)
                                } else if total_bytes >= 1024 {
                                    format!("{:.1} KB", total_bytes as f64 / 1024.0)
                                } else {
                                    format!("{} bytes", total_bytes)
                                };
                                rsx! {
                                    div { class: "grid grid-cols-2 gap-4",
                                        div { class: "card text-center",
                                            p { class: "label", "Blocks Stored" }
                                            p { class: "text-2xl font-bold mt-1", "{block_count}" }
                                        }
                                        div { class: "card text-center",
                                            p { class: "label", "Total Size" }
                                            p { class: "text-2xl font-bold mt-1", "{size_display}" }
                                        }
                                    }
                                    if block_count >= 10 {
                                        div { class: "mt-4 p-3 rounded-lg bg-[var(--bg-elevated)] border border-[var(--border-default)]",
                                            p { class: "text-sm text-[var(--text-secondary)]", "🌐 Contributing to network storage" }
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
