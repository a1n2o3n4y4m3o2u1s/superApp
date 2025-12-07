use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::{DagPayload, ProposalType, VoteType}};
use crate::components::AppState;

#[component]
pub fn GovernanceComponent() -> Element {
    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let mut show_create_modal = use_signal(|| false);
    let mut show_candidacy_modal = use_signal(|| false);
    let mut active_tab = use_signal(|| "proposals".to_string());
    
    // Form state for proposals
    let mut title = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());
    let mut proposal_type = use_signal(|| "Standard".to_string());
    let mut tax_rate = use_signal(|| 0i64);
    
    // Form state for candidacy
    let mut selected_ministry = use_signal(|| "VerificationAndIdentity".to_string());
    let mut platform = use_signal(|| "".to_string());

    // Form state for recall
    let mut show_recall_modal = use_signal(|| false);
    let mut recall_target = use_signal(|| "".to_string());
    let mut recall_reason = use_signal(|| "".to_string());
    let mut recall_ministry = use_signal(|| "VerificationAndIdentity".to_string());

    // Fetch data on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::FetchProposals);
        let _ = cmd_tx_effect.send(AppCmd::FetchCandidates);
        let _ = cmd_tx_effect.send(AppCmd::FetchReports);
        let _ = cmd_tx_effect.send(AppCmd::FetchRecalls);
        let _ = cmd_tx_effect.send(AppCmd::FetchOversightCases);
        let _ = cmd_tx_effect.send(AppCmd::FetchJuryDuty);
        let _ = cmd_tx_effect.send(AppCmd::FetchMinistries);
    });

    // Fetch tallies when proposals change
    let cmd_tx_proposals = cmd_tx.clone();
    use_effect(move || {
        let proposals = app_state.proposals.read();
        for node in proposals.iter() {
            let _ = cmd_tx_proposals.send(AppCmd::FetchProposalTally { proposal_id: node.id.clone() });
        }
    });

    // Fetch tallies when candidates change
    let cmd_tx_candidates = cmd_tx.clone();
    use_effect(move || {
        let candidates = app_state.candidates.read();
        for node in candidates.iter() {
            let _ = cmd_tx_candidates.send(AppCmd::FetchCandidateTally { candidacy_id: node.id.clone() });
        }
    });

    // Fetch recall tallies when recalls change
    let cmd_tx_recalls = cmd_tx.clone();
    use_effect(move || {
        let recalls = app_state.recalls.read();
        for node in recalls.iter() {
            let _ = cmd_tx_recalls.send(AppCmd::FetchRecallTally { recall_id: node.id.clone() });
        }
    });

    let cmd_tx_submit = cmd_tx.clone();
    let on_submit_proposal = move |_| {
        let p_type = match proposal_type().as_str() {
            "Constitutional" => ProposalType::Constitutional,
            "Emergency" => ProposalType::Emergency,
            "SetTax" => ProposalType::SetTax(tax_rate() as u8),
            _ => ProposalType::Standard,
        };

        let _ = cmd_tx_submit.send(AppCmd::PublishProposal {
            title: title(),
            description: description(),
            r#type: p_type,
        });

        title.set("".to_string());
        description.set("".to_string());
        show_create_modal.set(false);
    };

    let cmd_tx_candidacy = cmd_tx.clone();
    let on_submit_candidacy = move |_| {
        let ministry = selected_ministry();

        let _ = cmd_tx_candidacy.send(AppCmd::DeclareCandidacy {
            ministry,
            platform: platform(),
        });

        platform.set("".to_string());
        show_candidacy_modal.set(false);
    };

    let cmd_tx_recall = cmd_tx.clone();
    let on_submit_recall = move |_| {
        let ministry = recall_ministry();

        let _ = cmd_tx_recall.send(AppCmd::InitiateRecall {
            target_official: recall_target(),
            ministry,
            reason: recall_reason(),
        });

        recall_target.set("".to_string());
        recall_reason.set("".to_string());
        show_recall_modal.set(false);
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in flex flex-col min-h-[calc(100vh-64px)]",
            
            // Header
            div { class: "page-header",
                div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "page-title", "Governance Portal" }
                        p { class: "text-[var(--text-secondary)]", "Voice of the People: 1 Human = 1 Vote" }
                    }
                    div { class: "flex gap-2",
                         Link {
                            to: crate::Route::TransparencyComponent {},
                            class: "btn btn-secondary",
                            "👁️ Transparency"
                        }
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_: Event<MouseData>| show_candidacy_modal.set(true),
                            "🗳️ Run for Office"
                        }
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_: Event<MouseData>| show_recall_modal.set(true),
                            "⚠️ Recall Official"
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_: Event<MouseData>| show_create_modal.set(true),
                            "📝 Draft Proposal"
                        }
                    }
                }
            }

            // Tabs
            div { class: "flex gap-4 mb-6",
                button {
                    class: if active_tab() == "proposals" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("proposals".to_string()),
                    "📋 Proposals"
                }
                button {
                    class: if active_tab() == "elections" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("elections".to_string()),
                    "🏛️ Elections"
                }
                button {
                    class: if active_tab() == "recalls" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("recalls".to_string()),
                    "⚠️ Recalls"
                }
                button {
                    class: if active_tab() == "moderation" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("moderation".to_string()),
                    "🛡️ Moderation"
                }
            }

            // Content based on active tab
            if active_tab() == "proposals" {
                // Proposals List
                div { class: "grid gap-6",
                    {
                        let proposals = app_state.proposals.read();
                        if proposals.is_empty() {
                            rsx! {
                                div { class: "text-center py-10 text-[var(--text-muted)]",
                                    "No active proposals. Be the first to draft one!"
                                }
                            }
                        } else {
                            rsx! {
                                for node in proposals.iter() {
                                    if let DagPayload::Proposal(prop) = &node.payload {
                                        {
                                            let pid = node.id.clone();
                                            let cmd_tx_vote = cmd_tx.clone();
                                            
                                            let tallies_map = app_state.proposal_tallies.read();
                                            let (yes, no, abstain, petition, unique_voters, status) = tallies_map
                                                .get(&pid)
                                                .cloned()
                                                .unwrap_or((0, 0, 0, 0, 0, "Unknown".to_string()));
                                            
                                            let author_short = &node.author[0..8];
                                            let type_str = match &prop.r#type {
                                                ProposalType::SetTax(rate) => format!("Tax Rate: {}%", rate),
                                                ProposalType::DefineMinistries(_) => "Define Ministries".to_string(),
                                                ProposalType::Constitutional => "Constitutional".to_string(),
                                                ProposalType::Emergency => "Emergency".to_string(),
                                                ProposalType::Standard => "Standard".to_string(),
                                            };
                                            
                                            let status_color = match status.as_str() {
                                                "Passed" => "text-green-500",
                                                "Rejected" | "Failed (No votes)" => "text-red-500",
                                                s if s.starts_with("Voting") => "text-blue-500",
                                                s if s.starts_with("Petitioning") => "text-yellow-500",
                                                _ => "text-[var(--text-muted)]",
                                            };
                                            
                                            rsx! {
                                                div { key: "{pid}", class: "panel",
                                                    div { class: "flex justify-between items-start mb-4",
                                                        div {
                                                            div { class: "flex gap-2 items-center mb-2",
                                                                span { class: "px-2 py-0.5 rounded text-xs bg-[var(--bg-secondary)]", "{type_str}" }
                                                                span { class: "font-bold text-sm {status_color} border border-current px-2 py-0.5 rounded", "{status}" }
                                                            }
                                                            h2 { class: "text-xl font-bold", "{prop.title}" }
                                                            p { class: "text-xs text-[var(--text-muted)] mt-1", "Proposed by {author_short}..." }
                                                        }
                                                        div { class: "text-right",
                                                            div { class: "text-sm font-bold text-[var(--primary)]", "👥 {unique_voters} voters" }
                                                        }
                                                    }
                                                    
                                                    div { class: "prose max-w-none mb-6 text-[var(--text-secondary)]",
                                                        "{prop.description}"
                                                    }

                                                    div { class: "grid grid-cols-4 gap-2 mb-4 text-center text-sm",
                                                        div { class: "p-2 rounded bg-green-900/20",
                                                            div { class: "text-lg font-bold text-green-400", "✅ {yes}" }
                                                            div { class: "text-xs text-[var(--text-muted)]", "Yes" }
                                                        }
                                                        div { class: "p-2 rounded bg-red-900/20",
                                                            div { class: "text-lg font-bold text-red-400", "❌ {no}" }
                                                            div { class: "text-xs text-[var(--text-muted)]", "No" }
                                                        }
                                                        div { class: "p-2 rounded bg-gray-900/20",
                                                            div { class: "text-lg font-bold text-gray-400", "⏸️ {abstain}" }
                                                            div { class: "text-xs text-[var(--text-muted)]", "Abstain" }
                                                        }
                                                        div { class: "p-2 rounded bg-blue-900/20",
                                                            div { class: "text-lg font-bold text-blue-400", "✍️ {petition}" }
                                                            div { class: "text-xs text-[var(--text-muted)]", "Petitions" }
                                                        }
                                                    }

                                                    div { class: "flex flex-wrap gap-2 border-t border-[var(--border-color)] pt-4",
                                                        button {
                                                            class: "btn btn-secondary flex-1",
                                                            onclick: {
                                                                let cmd_tx = cmd_tx_vote.clone();
                                                                let pid = pid.clone();
                                                                move |_| {
                                                                    let _ = cmd_tx.send(AppCmd::VoteProposal { proposal_id: pid.clone(), vote: VoteType::PetitionSignature });
                                                                    let _ = cmd_tx.send(AppCmd::FetchProposalTally { proposal_id: pid.clone() });
                                                                }
                                                            },
                                                            "✍️ Sign"
                                                        }
                                                        button {
                                                            class: "btn flex-1",
                                                            style: "background: #22c55e; color: white;",
                                                            onclick: {
                                                                let cmd_tx = cmd_tx_vote.clone();
                                                                let pid = pid.clone();
                                                                move |_| {
                                                                    let _ = cmd_tx.send(AppCmd::VoteProposal { proposal_id: pid.clone(), vote: VoteType::Yes });
                                                                    let _ = cmd_tx.send(AppCmd::FetchProposalTally { proposal_id: pid.clone() });
                                                                }
                                                            },
                                                            "✅ Yes"
                                                        }
                                                        button {
                                                            class: "btn flex-1",
                                                            style: "background: #ef4444; color: white;",
                                                            onclick: {
                                                                let cmd_tx = cmd_tx_vote.clone();
                                                                let pid = pid.clone();
                                                                move |_| {
                                                                    let _ = cmd_tx.send(AppCmd::VoteProposal { proposal_id: pid.clone(), vote: VoteType::No });
                                                                    let _ = cmd_tx.send(AppCmd::FetchProposalTally { proposal_id: pid.clone() });
                                                                }
                                                            },
                                                            "❌ No"
                                                        }
                                                        button {
                                                            class: "btn flex-1",
                                                            style: "background: #6b7280; color: white;",
                                                            onclick: {
                                                                let cmd_tx = cmd_tx_vote.clone();
                                                                let pid = pid.clone();
                                                                move |_| {
                                                                    let _ = cmd_tx.send(AppCmd::VoteProposal { proposal_id: pid.clone(), vote: VoteType::Abstain });
                                                                    let _ = cmd_tx.send(AppCmd::FetchProposalTally { proposal_id: pid.clone() });
                                                                }
                                                            },
                                                            "⏸️ Abstain"
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
            } else if active_tab() == "elections" {
                // Elections Tab
                div { class: "grid gap-6",
                    {
                        let ministries_list = app_state.ministries.read();
                        rsx! {
                            for m_name in ministries_list.iter() {
                                div { class: "panel",
                                    h2 { class: "text-xl font-bold mb-4", "🏛️ Ministry of {m_name}" }
                                    {
                                        let candidates = app_state.candidates.read();
                                        let current_ministry_name = m_name.clone();
                                        let section_candidates: Vec<_> = candidates.iter()
                                            .filter(|n| {
                                                if let DagPayload::Candidacy(c) = &n.payload {
                                                    c.ministry == current_ministry_name
                                                } else { false }
                                            }).collect();
                                        
                                        if section_candidates.is_empty() {
                                            rsx! { p { class: "text-[var(--text-muted)]", "No candidates yet" } }
                                        } else {
                                            rsx! {
                                                for node in section_candidates {
                                                    if let DagPayload::Candidacy(c) = &node.payload {
                                                        {
                                                            let cid = node.id.clone();
                                                            let cmd_tx_cvote = cmd_tx.clone();
                                                            let tallies = app_state.candidate_tallies.read();
                                                            let votes = tallies.get(&cid).cloned().unwrap_or(0);
                                                            let author_short = &node.author[0..8];
                                                            rsx! {
                                                                div { key: "{cid}", class: "flex items-center justify-between p-3 bg-[var(--bg-secondary)] rounded-lg mb-2",
                                                                    div {
                                                                        div { class: "font-bold", "{author_short}..." }
                                                                        div { class: "text-sm text-[var(--text-muted)]", "{c.platform}" }
                                                                    }
                                                                    div { class: "flex items-center gap-3",
                                                                        div { class: "text-lg font-bold", "🗳️ {votes}" }
                                                                        button {
                                                                            class: "btn btn-primary btn-sm",
                                                                            onclick: {
                                                                                let cid = cid.clone();
                                                                                let cmd_tx = cmd_tx_cvote.clone();
                                                                                move |_| {
                                                                                    let _ = cmd_tx.send(AppCmd::VoteForCandidate { candidacy_id: cid.clone() });
                                                                                    let _ = cmd_tx.send(AppCmd::FetchCandidateTally { candidacy_id: cid.clone() });
                                                                                }
                                                                            },
                                                                            "Vote"
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
            } else if active_tab() == "recalls" {
                // Recalls Tab
                div { class: "grid gap-6",
                    div { class: "panel", 
                         h2 { class: "text-xl font-bold mb-4", "⚠️ Active Recall Campaigns" }
                         p { class: "text-[var(--text-secondary)] mb-6", "Citizens have the power to recall officials who fail in their duties. A recall requires a majority vote to pass." }

                         {
                             let recalls = app_state.recalls.read();
                             if recalls.is_empty() {
                                 rsx! { p { class: "text-[var(--text-muted)]", "No active recall campaigns." } }
                             } else {
                                 rsx! {
                                     for node in recalls.iter() {
                                         if let DagPayload::Recall(r) = &node.payload {
                                             {
                                                 let rid = node.id.clone();
                                                 let cmd_tx_rvote = cmd_tx.clone();
                                                 let tallies = app_state.recall_tallies.read();
                                                 let (remove, keep, unique) = tallies.get(&rid).cloned().unwrap_or((0, 0, 0));
                                                 let author_short = &node.author[0..8];
                                                 let target_short = &r.target_official[0..8];
                                                 
                                                 rsx! {
                                                     div { key: "{rid}", class: "panel border border-red-900/30 mb-4",
                                                         div { class: "flex justify-between items-start mb-2",
                                                             div {
                                                                 h3 { class: "font-bold text-lg text-red-400", "Recall: {target_short}..." }
                                                                 p { class: "text-xs text-[var(--text-muted)]", "Ministry: {r.ministry:?}" }
                                                                 p { class: "text-xs text-[var(--text-muted)]", "Initiated by {author_short}..." }
                                                             }
                                                             div { class: "text-right",
                                                                 div { class: "text-sm font-bold", "👥 {unique} voters" }
                                                             }
                                                         }
                                                         
                                                         p { class: "text-[var(--text-secondary)] mb-4 italic", "\"{r.reason}\"" }
                                                         
                                                         div { class: "grid grid-cols-2 gap-4 mb-4",
                                                            div { class: "p-2 rounded bg-red-900/20 text-center",
                                                                div { class: "text-xl font-bold text-red-500", "{remove}" }
                                                                div { class: "text-xs uppercase", "Remove" }
                                                            }
                                                            div { class: "p-2 rounded bg-green-900/20 text-center",
                                                                div { class: "text-xl font-bold text-green-500", "{keep}" }
                                                                div { class: "text-xs uppercase", "Keep" }
                                                            }
                                                         }
                                                         
                                                         div { class: "flex gap-2",
                                                             button {
                                                                 class: "btn flex-1 bg-red-600 hover:bg-red-700 text-white",
                                                                 onclick: {
                                                                     let rid = rid.clone();
                                                                     let cmd_tx = cmd_tx_rvote.clone();
                                                                     move |_| {
                                                                         let _ = cmd_tx.send(AppCmd::VoteRecall { recall_id: rid.clone(), vote: true });
                                                                         let _ = cmd_tx.send(AppCmd::FetchRecallTally { recall_id: rid.clone() });
                                                                     }
                                                                 },
                                                                 "🔥 Vote to Remove"
                                                             }
                                                             button {
                                                                 class: "btn flex-1 bg-green-600 hover:bg-green-700 text-white",
                                                                 onclick: {
                                                                     let rid = rid.clone();
                                                                     let cmd_tx = cmd_tx_rvote.clone();
                                                                     move |_| {
                                                                         let _ = cmd_tx.send(AppCmd::VoteRecall { recall_id: rid.clone(), vote: false });
                                                                         let _ = cmd_tx.send(AppCmd::FetchRecallTally { recall_id: rid.clone() });
                                                                     }
                                                                 },
                                                                 "🛡️ Vote to Keep"
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


            } else {
                // Moderation Tab
                div { class: "grid gap-6",
                    div { class: "panel",
                        h2 { class: "text-xl font-bold mb-4", "🚨 Decentralized Moderation Reports" }
                        p { class: "text-[var(--text-secondary)] mb-6", "Review reports submitted by the community. As a verified citizen, your vigilance helps keep the network safe." }
                        
                        {
                            let jury_duty = app_state.jury_duty.read();
                            if !jury_duty.is_empty() {
                                rsx! {
                                    div { class: "mb-8 p-4 bg-yellow-900/20 border border-yellow-500/30 rounded-lg",
                                        h3 { class: "font-bold text-yellow-400 mb-2", "⚖️ Jury Duty Assigned" }
                                        p { class: "text-sm text-[var(--text-muted)] mb-4", "You have been selected to serve on the following oversight cases." }
                                        for node in jury_duty.iter() {
                                            if let DagPayload::OversightCase(c) = &node.payload {
                                                {
                                                    let cid = c.case_id.clone();
                                                    let short_cid = if cid.len() > 8 { cid[0..8].to_string() } else { cid.clone() };
                                                    let display_cid = format!("Case #{}...", short_cid);
                                                    let cmd_tx_vote = cmd_tx.clone();
                                                    rsx! {
                                                        div { key: "{cid}", class: "bg-[var(--bg-secondary)] p-3 rounded mb-2",
                                                            div { class: "flex justify-between items-center",
                                                                span { "{display_cid}" }
                                                                div { class: "flex gap-2",
                                                                    button {
                                                                        class: "btn btn-sm bg-red-600 hover:bg-red-700 text-white",
                                                                        onclick: {
                                                                            let cid = cid.clone();
                                                                            let cmd_tx = cmd_tx_vote.clone();
                                                                            move |_| {
                                                                                let _ = cmd_tx.send(AppCmd::CastJuryVote { case_id: cid.clone(), vote: "Uphold".to_string() });
                                                                            }
                                                                        },
                                                                        "🔨 Uphold (Ban)"
                                                                    }
                                                                    button {
                                                                        class: "btn btn-sm bg-green-600 hover:bg-green-700 text-white",
                                                                        onclick: {
                                                                            let cid = cid.clone();
                                                                            let cmd_tx = cmd_tx_vote.clone();
                                                                            move |_| {
                                                                                let _ = cmd_tx.send(AppCmd::CastJuryVote { case_id: cid.clone(), vote: "Dismiss".to_string() });
                                                                            }
                                                                        },
                                                                        "Dismiss"
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
                            } else {
                                rsx! {}
                            }
                        }

                        {
                            let reports = app_state.reports.read();
                            if reports.is_empty() {
                                rsx! { p { class: "text-[var(--text-muted)]", "No active reports." } }
                            } else {
                                rsx! {
                                    for node in reports.iter() {
                                        if let DagPayload::Report(r) = &node.payload {
                                            {
                                                let rid = node.id.clone();
                                                let author_short = &node.author[0..8];
                                                rsx! {
                                                    div { key: "{rid}", class: "border-b border-[var(--border-default)] py-4 last:border-0",
                                                        div { class: "flex justify-between items-start",
                                                            div {
                                                                div { class: "flex items-center gap-2 mb-1",
                                                                    span { class: "px-2 py-0.5 rounded text-xs bg-red-900/20 text-red-400 font-bold", "{r.reason}" }
                                                                    span { class: "text-xs text-[var(--text-muted)]", "Reported by {author_short}..." }
                                                                }
                                                                div { class: "font-mono text-sm bg-[var(--bg-default)] p-1 rounded inline-block mb-2", "{r.target_id}" }
                                                                p { class: "text-sm text-[var(--text-secondary)]", "{r.details}" }
                                                            }
                                                            button {
                                                                class: "btn btn-sm btn-secondary",
                                                                onclick: {
                                                                    let rid = rid.clone();
                                                                    let cmd_tx = cmd_tx.clone();
                                                                    move |_| {
                                                                        let _ = cmd_tx.send(AppCmd::EscalateReport { report_id: rid.clone() });
                                                                    }
                                                                },
                                                                "⚖️ Escalate to Jury"
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
            
            // Create Proposal Modal
            if show_create_modal() {
                div { class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    div { class: "panel w-full max-w-2xl max-h-[90vh] overflow-y-auto",
                        div { class: "flex justify-between items-center mb-6",
                            h2 { class: "text-xl font-bold", "Draft New Proposal" }
                            button { 
                                class: "w-8 h-8 rounded-full bg-[var(--bg-secondary)] hover:bg-red-500/20 flex items-center justify-center text-[var(--text-muted)] hover:text-red-400 transition-colors text-lg font-bold",
                                onclick: move |_| show_create_modal.set(false),
                                "×"
                            }
                        }
                        
                        div { class: "grid gap-4",
                                div { class: "mb-4",
                                    label { class: "block text-sm font-medium mb-1", "Type" }
                                    select {
                                        class: "w-full p-2 rounded bg-[var(--bg-primary)] border border-[var(--border-color)]",
                                        value: "{proposal_type}",
                                        oninput: move |e| proposal_type.set(e.value()),
                                        option { value: "Standard", "Standard (1 week, 50%)" }
                                        option { value: "Constitutional", "Constitutional (1 week, 66%)" }
                                        option { value: "Emergency", "Emergency (48h, 50%, 5% Threshold)" }
                                        option { value: "SetTax", "Set System Tax Rate" }
                                    }
                                }
                                
                                {
                                    if proposal_type() == "SetTax" {
                                        rsx! {
                                             div { class: "mb-4",
                                                label { class: "block text-sm font-medium mb-1", "Tax Rate (%)" }
                                                input {
                                                    class: "w-full",
                                                    r#type: "range",
                                                    min: "0",
                                                    max: "100",
                                                    value: "{tax_rate}",
                                                    oninput: move |e| tax_rate.set(e.value().parse().unwrap_or(0)),
                                                }
                                                div { class: "text-right font-bold text-lg", "{tax_rate}%" }
                                             }
                                        }
                                    } else {
                                        rsx!({})
                                    }
                                }

                            div { class: "form-group",
                                label { class: "form-label", "Title" }
                                input {
                                    class: "input",
                                    value: "{title}",
                                    oninput: move |e| title.set(e.value()),
                                    placeholder: "e.g., Fund Development of Region X"
                                }
                            }

                            div { class: "form-group",
                                label { class: "form-label", "Description & Execution Plan" }
                                textarea {
                                    class: "input min-h-[200px]",
                                    value: "{description}",
                                    oninput: move |e| description.set(e.value()),
                                    placeholder: "Describe the proposal in detail..."
                                }
                            }

                            div { class: "flex justify-end gap-3 mt-4",
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| show_create_modal.set(false),
                                    "Cancel"
                                }
                                button {
                                    class: "btn btn-primary",
                                    onclick: on_submit_proposal,
                                    "Submit Proposal"
                                }
                            }
                        }
                    }
                }
            }

            // Declare Candidacy Modal
            if show_candidacy_modal() {
                div { class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    div { class: "panel w-full max-w-lg max-h-[90vh] overflow-y-auto",
                        div { class: "flex justify-between items-center mb-6",
                            h2 { class: "text-xl font-bold", "🗳️ Run for Office" }
                            button { 
                                class: "w-8 h-8 rounded-full bg-[var(--bg-secondary)] hover:bg-red-500/20 flex items-center justify-center text-[var(--text-muted)] hover:text-red-400 transition-colors text-lg font-bold",
                                onclick: move |_| show_candidacy_modal.set(false),
                                "×"
                            }
                        }
                        
                        div { class: "grid gap-4",
                            div { class: "form-group",
                                label { class: "form-label", "Ministry" }
                                select { 
                                    class: "input",
                                    value: "{selected_ministry}",
                                    onchange: move |e| selected_ministry.set(e.value()),
                                    {
                                        let ministries_list = app_state.ministries.read();
                                        rsx! {
                                            for m in ministries_list.iter() {
                                                option { value: "{m}", "{m}" }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "form-group",
                                label { class: "form-label", "Your Platform" }
                                textarea {
                                    class: "input min-h-[150px]",
                                    value: "{platform}",
                                    oninput: move |e| platform.set(e.value()),
                                    placeholder: "Describe your vision and what you'll do in office..."
                                }
                            }

                            div { class: "flex justify-end gap-3 mt-4",
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| show_candidacy_modal.set(false),
                                    "Cancel"
                                }
                                button {
                                    class: "btn btn-primary",
                                    onclick: on_submit_candidacy,
                                    "Declare Candidacy"
                                }
                            }
                        }
                    }
                }
            }

            // Initiate Recall Modal
            if show_recall_modal() {
                div { class: "fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4",
                    div { class: "panel w-full max-w-lg max-h-[90vh] overflow-y-auto border-red-500/30",
                        div { class: "flex justify-between items-center mb-6",
                            h2 { class: "text-xl font-bold text-red-400", "⚠️ Initiate Recall" }
                            button { 
                                class: "w-8 h-8 rounded-full bg-[var(--bg-secondary)] hover:bg-red-500/20 flex items-center justify-center text-[var(--text-muted)] hover:text-red-400 transition-colors text-lg font-bold",
                                onclick: move |_| show_recall_modal.set(false),
                                "×"
                            }
                        }
                        
                        div { class: "grid gap-4",
                            div { class: "bg-red-900/20 p-4 rounded text-sm text-red-300 mb-2",
                                "Warning: Initiating a recall is a serious action. Frivolous recalls may negatively impact your reputation."
                            }

                            div { class: "form-group",
                                label { class: "form-label", "Target Official (Public Key)" }
                                input {
                                    class: "input",
                                    value: "{recall_target}",
                                    oninput: move |e| recall_target.set(e.value()),
                                    placeholder: "Hex public key of the official..."
                                }
                            }

                            div { class: "form-group",
                                label { class: "form-label", "Ministry" }
                                select { 
                                    class: "input",
                                    value: "{recall_ministry}",
                                    onchange: move |e| recall_ministry.set(e.value()),
                                    {
                                        let ministries_list = app_state.ministries.read();
                                        rsx! {
                                            for m in ministries_list.iter() {
                                                option { value: "{m}", "{m}" }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "form-group",
                                label { class: "form-label", "Reason for Recall" }
                                textarea {
                                    class: "input min-h-[100px]",
                                    value: "{recall_reason}",
                                    oninput: move |e| recall_reason.set(e.value()),
                                    placeholder: "Explain why this official should be removed..."
                                }
                            }

                            div { class: "flex justify-end gap-3 mt-4",
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| show_recall_modal.set(false),
                                    "Cancel"
                                }
                                button {
                                    class: "btn bg-red-600 hover:bg-red-700 text-white",
                                    onclick: on_submit_recall,
                                    "🔥 Initiate Recall"
                                }
                            }
                        }
                    }
                }
    }
}
}
}
