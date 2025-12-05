use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::DagPayload};
use crate::components::AppState;
use tokio::sync::mpsc::UnboundedSender;

#[component]
pub fn MessagingComponent() -> Element {
    let app_state = use_context::<AppState>();
    let mut input_msg = use_signal(|| String::new());
    let mut target_peer = use_signal(|| String::new());
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    
    let target = target_peer.read().clone();
    let local_id = app_state.local_peer_id.read().clone();

    let cmd_tx_effect = cmd_tx.clone();
    let target_effect = target.clone();
    use_effect(move || {
        let t = target_effect.clone();
        if !t.is_empty() {
            let _ = cmd_tx_effect.send(AppCmd::FetchMessages { peer_id: t });
        }
    });

    let messages_list = app_state.messages.read().iter().filter_map(|(msg, content)| {
        if let DagPayload::Message(payload) = &msg.payload {
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
    }).collect::<Vec<_>>();

    rsx! { 
        div { class: "flex flex-col min-h-screen",
            div { class: "flex-1 page-container py-8 animate-fade-in",
                div { class: "flex gap-6 h-[calc(100vh-128px)]",
                    
                    // Sidebar
                    div { class: "w-80 flex-shrink-0 panel flex flex-col",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "Messages" }
                        }
                        
                        // Identity card
                        div { class: "card mb-4",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "label", "Your Identity" }
                                span { class: "w-2 h-2 rounded-full bg-green-500" }
                            }
                            p { class: "text-xs font-mono text-[var(--text-muted)] truncate", "{local_id}" }
                        }
                        
                        // Search
                        div { class: "form-group",
                            input {
                                class: "input",
                                placeholder: "Enter Peer ID...",
                                value: "{target_peer}",
                                oninput: move |evt| target_peer.set(evt.value())
                            }
                        }
                        
                        // Peer list
                        div { class: "flex-1 overflow-y-auto",
                            p { class: "label mb-3", "Discovered Peers" }
                            {
                                app_state.peers.read().iter()
                                    .filter(|peer| **peer != local_id)
                                    .map(|peer| {
                                        let peer_clone = peer.clone();
                                        let is_active = *peer == target;
                                        
                                        rsx! {
                                            div {
                                                class: if is_active { "list-item active" } else { "list-item" },
                                                onclick: move |_| target_peer.set(peer_clone.clone()),
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
                        if target.is_empty() {
                            div { class: "flex-1 empty-state",
                                div { class: "empty-state-icon", "💬" }
                                p { class: "empty-state-title", "Select a peer to start chatting" }
                                p { class: "empty-state-text", "Choose from the sidebar or enter a Peer ID" }
                            }
                        } else {
                            // Chat header
                            div { class: "p-4 border-b border-[var(--border-default)] flex items-center gap-3",
                                div { class: "avatar" }
                                div { class: "flex-1",
                                    div { class: "flex items-center gap-2",
                                        p { class: "font-semibold", "Chat" }
                                        span { class: "badge badge-online text-xs", "🔒 Encrypted" }
                                    }
                                    p { class: "text-xs text-[var(--text-muted)] font-mono truncate", "{target}" }
                                }
                                Link {
                                    to: crate::Route::UserProfileComponent { peer_id: target.clone() },
                                    class: "btn btn-secondary btn-sm",
                                    "View Profile"
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
                                                        if !content.is_empty() && !recipient.is_empty() {
                                                            cmd_tx_key.send(AppCmd::SendMessage { recipient, content }).unwrap();
                                                            input_msg.set(String::new());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        let cmd_tx_click = cmd_tx.clone();
                                        rsx! {
                                            button {
                                                class: "btn btn-primary",
                                                onclick: move |_| {
                                                    let content = input_msg.read().clone();
                                                    let recipient = target_peer.read().clone();
                                                    if !content.is_empty() && !recipient.is_empty() {
                                                        cmd_tx_click.send(AppCmd::SendMessage { recipient, content }).unwrap();
                                                        input_msg.set(String::new());
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
    }
}

#[component]
fn MessageItem(msg: crate::backend::dag::DagNode, is_me: bool, content: String) -> Element {
    rsx! {
        div {
            class: if is_me { "flex flex-col items-end" } else { "flex flex-col items-start" },
            div {
                class: if is_me { "message-bubble sent" } else { "message-bubble received" },
                "{content}"
            }
            span { class: "message-time", "{msg.timestamp}" }
        }
    }
}