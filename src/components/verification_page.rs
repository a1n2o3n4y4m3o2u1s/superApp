use dioxus::prelude::*;
use crate::backend::{AppCmd, VerificationStatus, dag::DagPayload};

#[component]
pub fn VerificationPage() -> Element {
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let app_state = use_context::<crate::components::AppState>();

    let check_status = {
        let cmd_tx = cmd_tx.clone();
        move |_| {
            let _ = cmd_tx.send(AppCmd::CheckVerificationStatus);
        }
    };

    // Application form signals
    let mut name = use_signal(|| "".to_string());
    let mut bio = use_signal(|| "".to_string());
    let mut photo_cid = use_signal(|| None::<String>);

    // Fetch pending applications on mount for verified users
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::FetchPendingApplications);
        let _ = cmd_tx_effect.send(AppCmd::CheckVerificationStatus);
    });

    let verification_status = app_state.verification_status.read().clone();
    let is_verified = matches!(verification_status, VerificationStatus::Verified | VerificationStatus::Founder);
    let pending_apps = app_state.pending_applications.read().clone();

    // Submit application handler
    let cmd_tx_submit = cmd_tx.clone();
    let submit_application = move |_| {
        let n = name.read().clone();
        let b = bio.read().clone();
        let p = photo_cid.read().clone();
        if !n.is_empty() {
            let _ = cmd_tx_submit.send(AppCmd::SubmitApplication { name: n, bio: b, photo_cid: p });
        }
    };

    // Founder claim handler
    let cmd_tx_founder = cmd_tx.clone();
    let claim_founder = move |_| {
        let n = name.read().clone();
        let b = bio.read().clone();
        if !n.is_empty() {
            let _ = cmd_tx_founder.send(AppCmd::PublishProfile { name: n, bio: b, photo: None });
            let cmd_tx = cmd_tx_founder.clone();
            spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                let _ = cmd_tx.send(AppCmd::CheckVerificationStatus);
            });
        }
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Header
            div { class: "page-header mb-8",
                h1 { class: "page-title", "Verification" }
                p { class: "text-[var(--text-secondary)]", 
                    "Human verification ensures network integrity" 
                }
            }

            // Status Badge
            div { class: "mb-6 flex items-center gap-2",
                span { class: "text-sm text-[var(--text-muted)]", "Status:" }
                match verification_status {
                    VerificationStatus::Founder => rsx!{ span { class: "badge badge-founder", "🏆 Founder" } },
                    VerificationStatus::Verified => rsx!{ span { class: "badge badge-success", "✓ Verified" } },
                    VerificationStatus::Unverified => rsx!{ span { class: "badge bg-red-500/20 text-red-400", "Unverified" } },
                    VerificationStatus::EligibleForFounder => rsx!{ span { class: "badge bg-green-500/20 text-green-400", "Eligible for Founder" } },
                }
            }

            div { class: "grid grid-cols-1 lg:grid-cols-2 gap-6",
                
                // Left: Application / Status
                div { class: "panel",
                    if verification_status == VerificationStatus::EligibleForFounder {
                        // Founder claim
                        div { class: "panel-header",
                            h2 { class: "panel-title", "🎉 Claim Founder Status" }
                        }
                        p { class: "text-[var(--text-secondary)] mb-4", 
                            "You are one of the first 100 users! Claim your Founder status now." 
                        }
                        div { class: "space-y-4",
                            div { class: "form-group",
                                label { class: "form-label", "Name" }
                                input {
                                    class: "input",
                                    placeholder: "Your Name",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Bio" }
                                input {
                                    class: "input",
                                    placeholder: "Short Bio",
                                    value: "{bio}",
                                    oninput: move |e| bio.set(e.value())
                                }
                            }
                            button { 
                                class: "btn btn-primary w-full",
                                onclick: claim_founder,
                                "Claim Founder Status"
                            }
                        }
                    } else if verification_status == VerificationStatus::Unverified {
                        // Application form for unverified users
                        div { class: "panel-header",
                            h2 { class: "panel-title", "📝 Submit Verification Application" }
                        }
                        p { class: "text-[var(--text-secondary)] mb-4", 
                            "Submit your application to be reviewed by verified members of the network." 
                        }
                        div { class: "space-y-4",
                            div { class: "form-group",
                                label { class: "form-label", "Name *" }
                                input {
                                    class: "input",
                                    placeholder: "Your real name",
                                    value: "{name}",
                                    oninput: move |e| name.set(e.value())
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Bio" }
                                textarea {
                                    class: "input",
                                    style: "min-height: 80px;",
                                    placeholder: "Tell us about yourself...",
                                    value: "{bio}",
                                    oninput: move |e| bio.set(e.value())
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Photo CID (optional)" }
                                input {
                                    class: "input",
                                    placeholder: "Upload a photo first, then paste the CID",
                                    value: "{photo_cid().unwrap_or_default()}",
                                    oninput: move |e| {
                                        let v = e.value();
                                        if v.is_empty() { photo_cid.set(None); } 
                                        else { photo_cid.set(Some(v)); }
                                    }
                                }
                                p { class: "text-xs text-[var(--text-muted)] mt-1", 
                                    "Upload a clear photo/selfie to Profile → Files first" 
                                }
                            }
                            button { 
                                class: "btn btn-primary w-full",
                                onclick: submit_application,
                                disabled: name().is_empty(),
                                "Submit Application"
                            }
                        }
                        
                        div { class: "mt-6 pt-6 border-t border-[var(--border-subtle)]",
                            p { class: "text-xs text-[var(--text-muted)] mb-2", "Your Peer ID:" }
                            p { class: "font-mono text-xs break-all text-[var(--text-primary)] select-all", 
                                "{app_state.local_peer_id}" 
                            }
                        }
                    } else {
                        // Verified user info
                        div { class: "panel-header",
                            h2 { class: "panel-title", "✓ You are Verified" }
                        }
                        p { class: "text-[var(--text-secondary)]", 
                            "You can vote on pending applications to help grow the network." 
                        }
                    }
                }

                // Right: Accept Applications (for verified users)
                if is_verified {
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "📋 Accept Applications" }
                            button { 
                                class: "btn btn-sm btn-secondary",
                                onclick: move |_| { let _ = cmd_tx.send(AppCmd::FetchPendingApplications); },
                                "Refresh"
                            }
                        }
                        
                        // Info about required approvals
                        div { class: "mb-4 p-3 bg-blue-500/10 border border-blue-500/20 rounded-lg",
                            p { class: "text-sm text-blue-300", 
                                "ℹ️ Applicants need multiple approvals to become verified. The threshold scales with network size (1-10 votes)." 
                            }
                            p { class: "text-xs text-[var(--text-muted)] mt-1", 
                                "You can vote once per 12 hours." 
                            }
                        }
                        
                        if pending_apps.is_empty() {
                            div { class: "empty-state py-8",
                                p { class: "text-4xl mb-2", "✨" }
                                p { class: "empty-state-text", "No pending applications" }
                            }
                        } else {
                            div { class: "space-y-4",
                                for node in pending_apps.iter() {
                                    if let DagPayload::Application(ref app) = node.payload {
                                        {
                                            let app_id = node.id.clone();
                                            let app_name = app.name.clone();
                                            let app_bio = app.bio.clone();
                                            let app_photo = app.photo_cid.clone();
                                            let applicant = node.author.clone();
                                            let short_id = if app_id.len() > 8 { &app_id[0..8] } else { &app_id };
                                            let cmd_tx_approve = cmd_tx.clone();
                                            let cmd_tx_reject = cmd_tx.clone();
                                            let approve_id = app_id.clone();
                                            let reject_id = app_id.clone();
                                            
                                            rsx! {
                                                div { class: "card p-4 border border-[var(--border-color)]",
                                                    div { class: "flex gap-4",
                                                        // Photo
                                                        if let Some(ref cid) = app_photo {
                                                            div { class: "w-20 h-20 rounded-lg overflow-hidden bg-[var(--bg-elevated)] flex-shrink-0",
                                                                crate::components::home_page::BlobImage { 
                                                                    cid: cid.clone(), 
                                                                    class: "w-full h-full object-cover" 
                                                                }
                                                            }
                                                        } else {
                                                            div { class: "w-20 h-20 rounded-lg bg-[var(--bg-elevated)] flex items-center justify-center flex-shrink-0",
                                                                span { class: "text-3xl", "👤" }
                                                            }
                                                        }
                                                        
                                                        // Info
                                                        div { class: "flex-1 min-w-0",
                                                            p { class: "font-bold text-lg truncate", "{app_name}" }
                                                            p { class: "text-sm text-[var(--text-secondary)] line-clamp-2", "{app_bio}" }
                                                            p { class: "text-xs text-[var(--text-muted)] mt-1 truncate", 
                                                                "Applicant: {applicant.get(0..12).unwrap_or(&applicant)}..." 
                                                            }
                                                        }
                                                    }
                                                    
                                                    // Vote buttons
                                                    div { class: "flex gap-2 mt-4",
                                                        button { 
                                                            class: "btn btn-success flex-1",
                                                            onclick: move |_| {
                                                                let _ = cmd_tx_approve.send(AppCmd::VoteApplication { 
                                                                    application_id: approve_id.clone(), 
                                                                    approve: true 
                                                                });
                                                            },
                                                            "✓ Approve"
                                                        }
                                                        button { 
                                                            class: "btn bg-red-500/20 text-red-400 hover:bg-red-500/30 flex-1",
                                                            onclick: move |_| {
                                                                let _ = cmd_tx_reject.send(AppCmd::VoteApplication { 
                                                                    application_id: reject_id.clone(), 
                                                                    approve: false 
                                                                });
                                                            },
                                                            "✗ Reject"
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
            
            // Check status button at bottom
            div { class: "mt-6",
                button { 
                    class: "btn btn-secondary",
                    onclick: check_status,
                    "🔄 Check Status"
                }
            }
        }
    }
}
