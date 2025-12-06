use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::{DagPayload, ProposalType, VoteType, Ministry}};
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
    
    // Form state for candidacy
    let mut selected_ministry = use_signal(|| "VerificationAndIdentity".to_string());
    let mut platform = use_signal(|| "".to_string());

    // Fetch data on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::FetchProposals);
        let _ = cmd_tx_effect.send(AppCmd::FetchCandidates);
        let _ = cmd_tx_effect.send(AppCmd::FetchReports);
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

    let cmd_tx_submit = cmd_tx.clone();
    let on_submit_proposal = move |_| {
        let p_type = match proposal_type().as_str() {
            "Constitutional" => ProposalType::Constitutional,
            "Emergency" => ProposalType::Emergency,
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
        let ministry = match selected_ministry().as_str() {
            "TreasuryAndDistribution" => Ministry::TreasuryAndDistribution,
            "NetworkAndProtocols" => Ministry::NetworkAndProtocols,
            _ => Ministry::VerificationAndIdentity,
        };

        let _ = cmd_tx_candidacy.send(AppCmd::DeclareCandidacy {
            ministry,
            platform: platform(),
        });

        platform.set("".to_string());
        show_candidacy_modal.set(false);
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
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_: Event<MouseData>| show_candidacy_modal.set(true),
                            "🗳️ Run for Office"
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
                                            let (yes, no, abstain, petition, unique_voters) = tallies_map
                                                .get(&pid)
                                                .cloned()
                                                .unwrap_or((0, 0, 0, 0, 0));
                                            
                                            let author_short = &node.author[0..8];
                                            let type_str = format!("{:?}", prop.r#type);
                                            
                                            rsx! {
                                                div { key: "{pid}", class: "panel",
                                                    div { class: "flex justify-between items-start mb-4",
                                                        div {
                                                            span { class: "px-2 py-0.5 rounded text-xs bg-[var(--bg-secondary)] mb-2 inline-block", "{type_str}" }
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
                    // Ministry of Verification
                    div { class: "panel",
                        h2 { class: "text-xl font-bold mb-4", "🔐 Ministry of Verification & Identity" }
                        p { class: "text-sm text-[var(--text-muted)] mb-4", "3 positions • 6-month terms" }
                        {
                            let candidates = app_state.candidates.read();
                            let verification_candidates: Vec<_> = candidates.iter()
                                .filter(|n| {
                                    if let DagPayload::Candidacy(c) = &n.payload {
                                        c.ministry == Ministry::VerificationAndIdentity
                                    } else { false }
                                }).collect();
                            
                            if verification_candidates.is_empty() {
                                rsx! { p { class: "text-[var(--text-muted)]", "No candidates yet" } }
                            } else {
                                rsx! {
                                    for node in verification_candidates {
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

                    // Ministry of Treasury
                    div { class: "panel",
                        h2 { class: "text-xl font-bold mb-4", "💰 Ministry of Treasury & Distribution" }
                        p { class: "text-sm text-[var(--text-muted)] mb-4", "3 positions • 6-month terms" }
                        {
                            let candidates = app_state.candidates.read();
                            let treasury_candidates: Vec<_> = candidates.iter()
                                .filter(|n| {
                                    if let DagPayload::Candidacy(c) = &n.payload {
                                        c.ministry == Ministry::TreasuryAndDistribution
                                    } else { false }
                                }).collect();
                            
                            if treasury_candidates.is_empty() {
                                rsx! { p { class: "text-[var(--text-muted)]", "No candidates yet" } }
                            } else {
                                rsx! {
                                    for node in treasury_candidates {
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

                    // Ministry of Network
                    div { class: "panel",
                        h2 { class: "text-xl font-bold mb-4", "🌐 Ministry of Network & Protocols" }
                        p { class: "text-sm text-[var(--text-muted)] mb-4", "5 positions • 8-month terms" }
                        {
                            let candidates = app_state.candidates.read();
                            let network_candidates: Vec<_> = candidates.iter()
                                .filter(|n| {
                                    if let DagPayload::Candidacy(c) = &n.payload {
                                        c.ministry == Ministry::NetworkAndProtocols
                                    } else { false }
                                }).collect();
                            
                            if network_candidates.is_empty() {
                                rsx! { p { class: "text-[var(--text-muted)]", "No candidates yet" } }
                            } else {
                                rsx! {
                                    for node in network_candidates {
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
            } else {
                // Moderation Tab
                div { class: "grid gap-6",
                    div { class: "panel",
                        h2 { class: "text-xl font-bold mb-4", "🚨 Decentralized Moderation Reports" }
                        p { class: "text-[var(--text-secondary)] mb-6", "Review reports submitted by the community. As a verified citizen, your vigilance helps keep the network safe." }
                        
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
                                                            // Future: Action buttons (Ignore, Uphold, etc.)
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
                                class: "text-[var(--text-muted)] hover:text-[var(--text-primary)]",
                                onclick: move |_| show_create_modal.set(false),
                                "✕"
                            }
                        }
                        
                        div { class: "grid gap-4",
                            div { class: "form-group",
                                label { class: "form-label", "Proposal Type" }
                                select { 
                                    class: "input",
                                    value: "{proposal_type}",
                                    onchange: move |e| proposal_type.set(e.value()),
                                    option { value: "Standard", "Standard (Simple Majority)" }
                                    option { value: "Constitutional", "Constitutional (Supermajority)" }
                                    option { value: "Emergency", "Emergency (Fast Track)" }
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
                                class: "text-[var(--text-muted)] hover:text-[var(--text-primary)]",
                                onclick: move |_| show_candidacy_modal.set(false),
                                "✕"
                            }
                        }
                        
                        div { class: "grid gap-4",
                            div { class: "form-group",
                                label { class: "form-label", "Ministry" }
                                select { 
                                    class: "input",
                                    value: "{selected_ministry}",
                                    onchange: move |e| selected_ministry.set(e.value()),
                                    option { value: "VerificationAndIdentity", "🔐 Verification & Identity (3 seats)" }
                                    option { value: "TreasuryAndDistribution", "💰 Treasury & Distribution (3 seats)" }
                                    option { value: "NetworkAndProtocols", "🌐 Network & Protocols (5 seats)" }
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
        }
    }
}
