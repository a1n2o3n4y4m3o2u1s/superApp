use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::backend::dag::DagPayload;

#[component]
pub fn TransparencyComponent() -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    // Fetch ledger on mount
    let cmd_tx_clone = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_clone.send(AppCmd::FetchPublicLedger);
        let _ = cmd_tx_clone.send(AppCmd::FetchStorageStats);
    });
    
    // Periodically refresh
    use_future(move || {
        let cmd_tx_refresh = cmd_tx.clone();
        async move {
            loop {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(std::time::Duration::from_secs(10)).await;
                
                let _ = cmd_tx_refresh.send(AppCmd::FetchPublicLedger);
                let _ = cmd_tx_refresh.send(AppCmd::FetchStorageStats);
            }
        }
    });

    let (block_count, total_bytes) = (app_state.storage_stats)();
    let size_display = if total_bytes >= 1048576 {
        format!("{:.2} MB", total_bytes as f64 / 1048576.0)
    } else if total_bytes >= 1024 {
        format!("{:.1} KB", total_bytes as f64 / 1024.0)
    } else {
        format!("{} bytes", total_bytes)
    };
    
    let ledger_events = app_state.public_ledger.read();

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            // Header
            div { class: "page-header mb-8",
                h1 { class: "page-title", "Transparency Dashboard" }
                p { class: "text-[var(--text-secondary)]", "Real-time public ledger of all network actions and treasury flows." }
            }

            // Stats Grid
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-6 mb-8",
                div { class: "card flex items-center justify-between p-6",
                    div {
                        p { class: "label mb-1", "Total Blocks Stored" }
                        p { class: "text-3xl font-bold text-[var(--text-primary)]", "{block_count}" }
                    }
                    div { class: "text-4xl opacity-20", "📦" }
                }
                div { class: "card flex items-center justify-between p-6",
                    div {
                        p { class: "label mb-1", "Network Storage" }
                        p { class: "text-3xl font-bold text-[var(--text-primary)]", "{size_display}" }
                    }
                    div { class: "text-4xl opacity-20", "💾" }
                }
            }
            
            // Ledger Table
            div { class: "panel",
                div { class: "panel-header border-b border-[var(--border-default)] pb-4 mb-4",
                    h2 { class: "panel-title", "Public Ledger" }
                }
                
                if ledger_events.is_empty() {
                    div { class: "empty-state py-12",
                        p { class: "empty-state-text", "No public events recorded yet" }
                    }
                } else {
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-left border-collapse",
                            thead {
                                tr { class: "border-b border-[var(--border-default)] text-[var(--text-secondary)] text-sm",
                                    th { class: "py-3 px-4 font-medium", "Time" }
                                    th { class: "py-3 px-4 font-medium", "Type" }
                                    th { class: "py-3 px-4 font-medium", "Actor" }
                                    th { class: "py-3 px-4 font-medium", "Details" }
                                }
                            }
                            tbody {
                                for node in ledger_events.iter() {
                                    {
                                        let timestamp = node.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
                                        let actor = format!("{}...", &node.author[0..8]);
                                        
                                        let (type_badge, details) = match &node.payload {
                                            DagPayload::Token(t) => {
                                                let action = format!("{:?}", t.action);
                                                let amt = format!("{} SUPER", t.amount);
                                                (
                                                    rsx!{ span { class: "badge badge-primary", "Token: {action}" } },
                                                    rsx!{ span { "{amt}" } }
                                                )
                                            },
                                            DagPayload::Proposal(p) => {
                                                (
                                                    rsx!{ span { class: "badge badge-accent", "Proposal" } },
                                                    rsx!{ span { "{p.title}" } }
                                                )
                                            },
                                            DagPayload::Vote(v) => {
                                                let vote_type = format!("{:?}", v.vote);
                                                (
                                                    rsx!{ span { class: "badge badge-secondary", "Vote" } },
                                                    rsx!{ span { "Proposal {&v.proposal_id[0..8]}...: {vote_type}" } }
                                                )
                                            },
                                            DagPayload::Candidacy(c) => {
                                                let ministry = format!("{:?}", c.ministry);
                                                (
                                                    rsx!{ span { class: "badge badge-verified", "Candidacy" } },
                                                    rsx!{ span { "{ministry}" } }
                                                )
                                            },
                                             DagPayload::Contract(c) => {
                                                (
                                                    rsx!{ span { class: "badge bg-purple-500/20 text-purple-400 border-purple-500/30", "Contract" } },
                                                    rsx!{ span { "New Deployment" } }
                                                )
                                            },
                                            DagPayload::ContractCall(c) => {
                                                 (
                                                    rsx!{ span { class: "badge bg-purple-500/10 text-purple-400 border-purple-500/20", "Call" } },
                                                    rsx!{ span { "{c.method}" } }
                                                )
                                            },
                                            _ => (rsx!{ span { class: "badge", "Event" } }, rsx!{ span { "-" } }),
                                        };
    
                                        rsx! {
                                            tr { class: "border-b border-[var(--border-default)] hover:bg-[var(--bg-elevated)] transition-colors",
                                                td { class: "py-3 px-4 text-sm font-mono text-[var(--text-muted)]", "{timestamp}" }
                                                td { class: "py-3 px-4", {type_badge} }
                                                td { class: "py-3 px-4 text-sm font-mono text-[var(--primary)]", "{actor}" }
                                                td { class: "py-3 px-4 text-sm", {details} }
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
