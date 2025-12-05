use dioxus::prelude::*;
use crate::backend::{AppCmd, VerificationStatus};

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

    let mut name = use_signal(|| "".to_string());
    let mut bio = use_signal(|| "".to_string());

    let claim_founder = {
        let cmd_tx = cmd_tx.clone();
        move |_| {
            let n = name.read().clone();
            let b = bio.read().clone();
            if !n.is_empty() {
                let _ = cmd_tx.send(AppCmd::PublishProfile { name: n, bio: b });
                let cmd_tx = cmd_tx.clone();
                spawn(async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    #[cfg(target_arch = "wasm32")]
                    gloo_timers::future::sleep(std::time::Duration::from_secs(2)).await;
                    let _ = cmd_tx.send(AppCmd::CheckVerificationStatus);
                });
            }
        }
    };

    rsx! {
        div {
            class: "flex flex-col min-h-screen",
            div {
                class: "page-container py-8 animate-fade-in flex flex-col items-center justify-center min-h-[80vh]",
                
                div {
                    class: "glass-panel p-8 max-w-lg w-full text-center relative overflow-hidden",
                    
                    // Icon
                    div { class: "mb-6 text-6xl", "🔒" }

                    h1 { class: "text-3xl font-bold mb-2 text-white", "Verification Required" }
                    
                    if *app_state.verification_status.read() == VerificationStatus::EligibleForFounder {
                        div { class: "mb-6",
                            p { class: "text-green-400 font-bold text-lg mb-2", "🎉 You are one of the first 100 users!" }
                            p { class: "text-gray-300 mb-6", "You can claim Founder status instantly." }
                            
                            div { class: "space-y-4 text-left",
                                div {
                                    input {
                                        class: "glass-input w-full",
                                        placeholder: "Your Name",
                                        value: "{name}",
                                        oninput: move |e| name.set(e.value())
                                    }
                                }
                                div {
                                    input {
                                        class: "glass-input w-full",
                                        placeholder: "Short Bio",
                                        value: "{bio}",
                                        oninput: move |e| bio.set(e.value())
                                    }
                                }
                                
                                div { class: "pt-2",
                                    button {
                                        class: "glass-button w-full justify-center", // Full width button
                                        onclick: claim_founder,
                                        "Claim Founder Status"
                                    }
                                }
                            }
                        }
                    } else {
                        p { class: "text-gray-300 mb-8 leading-relaxed",
                            "You are User #101+. To maintain network integrity, new users must be verified by an existing member."
                        }

                        div { class: "bg-white/5 p-4 rounded-lg border border-white/10 mb-8 text-left",
                            p { class: "text-xs text-gray-500 uppercase font-bold mb-2", "Your Peer ID" }
                            p { class: "font-mono text-xs break-all text-blue-300 select-all leading-relaxed", "{app_state.local_peer_id}" }
                            p { class: "text-[10px] text-gray-500 mt-3", "Share this ID with a verified friend to get vouched." }
                        }

                        button {
                            class: "glass-button w-full justify-center",
                            onclick: check_status,
                            "Check Verification Status"
                        }
                    }
                    
                    div { class: "mt-6 pt-6 border-t border-white/10 text-xs text-gray-500",
                        span { class: "mr-2", "Current Status:" }
                        match *app_state.verification_status.read() {
                            VerificationStatus::Founder => rsx!{ span { class: "badge-founder", "Founder" } },
                            VerificationStatus::Verified => rsx!{ span { class: "badge-verified", "Verified" } },
                            VerificationStatus::Unverified => rsx!{ span { class: "text-red-400 font-bold", "Unverified" } },
                            VerificationStatus::EligibleForFounder => rsx!{ span { class: "text-green-400 font-bold", "Eligible" } },
                        }
                    }
                }
            }
        }
    }
}
