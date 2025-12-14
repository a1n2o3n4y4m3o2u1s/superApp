use dioxus::prelude::*;
use crate::components::AppState;
use crate::backend::AppCmd;
use tokio::sync::mpsc::UnboundedSender;
use crate::backend::dag::{DagPayload, ContractCallPayload};
use crate::backend::dag::DagNode;
use serde_json;

#[component]
pub fn SmartContractsPage() -> Element {
    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    
    // Clones for closures
    let cmd_tx_create = cmd_tx.clone();
    
    // State
    let mut show_create_wizard = use_signal(|| false);
    let mut selected_contract_id = use_signal(|| None::<String>);

    // Fetch contracts on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        cmd_tx_effect.send(AppCmd::FetchContracts);
        cmd_tx_effect.send(AppCmd::FetchPendingContracts);
    });

    let cmd_tx_accept = cmd_tx.clone();
    let cmd_tx_reject = cmd_tx.clone();

    rsx! {
        div { class: "flex flex-col h-full bg-base-200 p-4",
            div { class: "flex justify-between items-center mb-6",
                h1 { class: "text-2xl font-bold", "Smart Agreements" }
                button { 
                    class: "btn btn-primary",
                    onclick: move |_| show_create_wizard.set(true),
                    "New Agreement"
                }
            }

            // Pending Contracts Section
            if !app_state.pending_contracts.read().is_empty() {
                div { class: "card bg-warning/20 border-2 border-warning shadow-xl mb-6",
                    div { class: "card-body",
                        h3 { class: "card-title text-warning", "⏳ Pending Agreements" }
                        p { class: "text-sm opacity-75 mb-4", "These agreements require your acceptance" }
                        for node in app_state.pending_contracts.read().iter() {
                            if let DagPayload::Contract(c) = &node.payload {
                                {
                                    let params: serde_json::Value = serde_json::from_str(&c.init_params).unwrap_or(serde_json::json!({}));
                                    let title = params["metadata"]["title"].as_str().unwrap_or("Untitled");
                                    let type_label = params["metadata"]["type_label"].as_str().unwrap_or("Contract");
                                    let node_id = node.id.clone();
                                    let node_id_reject = node.id.clone();
                                    let cmd_tx_a = cmd_tx_accept.clone();
                                    let cmd_tx_r = cmd_tx_reject.clone();
                                    rsx! {
                                        div { class: "flex justify-between items-center p-3 bg-base-100 rounded-lg mb-2",
                                            div {
                                                span { class: "font-semibold", "{title}" }
                                                span { class: "badge badge-outline ml-2", "{type_label}" }
                                            }
                                            div { class: "flex gap-2",
                                                button { 
                                                    class: "btn btn-success btn-sm",
                                                    onclick: move |_| { let _ = cmd_tx_a.send(AppCmd::AcceptContract { contract_id: node_id.clone() }); },
                                                    "Accept"
                                                }
                                                button { 
                                                    class: "btn btn-error btn-sm",
                                                    onclick: move |_| { let _ = cmd_tx_r.send(AppCmd::RejectContract { contract_id: node_id_reject.clone() }); },
                                                    "Reject"
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

            if *show_create_wizard.read() {
                ContractWizard { 
                    on_close: move |_| show_create_wizard.set(false),
                    on_create: move |_| {
                         show_create_wizard.set(false);
                         cmd_tx_create.send(AppCmd::FetchContracts);
                    }
                }
            } else if let Some(cid) = selected_contract_id.read().clone() {
                ContractDetail { 
                    contract_id: cid.clone(),
                    on_back: move |_| selected_contract_id.set(None)
                }
            } else {
                ContractList { 
                    on_select: move |cid: String| selected_contract_id.set(Some(cid))
                }
            }
        }
    }
}

#[component]
fn ContractList(on_select: EventHandler<String>) -> Element {
    let app_state = use_context::<AppState>();
    let contracts = app_state.contracts.read();
    
    // Filter contracts where I am relevant (MVP: just show all for now, or those I authored)
    // Actually, we should parse the init_params to see if my Key follows "employer", "employee", "tenant", etc.
    // For now, let's just list all contracts found in the DAG to verify functionality.
    
    rsx! {
        div { class: "grid gap-4",
            if contracts.is_empty() {
                div { class: "text-center opacity-50", "No active agreements found." }
            }
            for node in contracts.iter() {
                {
                    let payload = &node.payload;
                    if let DagPayload::Contract(c) = payload {
                        let params: serde_json::Value = serde_json::from_str(&c.init_params).unwrap_or(serde_json::json!({}));
                        
                        // Modular Logic: Try to get title from metadata
                        let title = params["metadata"]["title"].as_str()
                            .map(|s| s.to_string())
                            .or_else(|| {
                                // Fallback/Legacy
                                if c.init_params.contains("hourly_rate") { Some("Wages (Legacy)".to_string()) }
                                else if c.init_params.contains("monthly_rent") { Some("Rent (Legacy)".to_string()) }
                                else if c.init_params.contains("interest_rate") { Some("Loan (Legacy)".to_string()) }
                                else { Some("Untitled Agreement".to_string()) }
                            })
                            .unwrap();
                            
                        let type_label = params["metadata"]["type_label"].as_str().unwrap_or("Custom");
                            
                        let time_str = node.timestamp.format("%Y-%m-%d").to_string();

                        let node_id = node.id.clone();
                        rsx! {
                            div { 
                                class: "card bg-base-100 shadow-xl cursor-pointer hover:bg-base-200 transition-colors",
                                onclick: move |_| on_select.call(node_id.clone()),
                                div { class: "card-body",
                                    div { class: "flex justify-between",
                                        h3 { class: "card-title", "{title}" }
                                        span { class: "badge badge-outline", "{time_str}" }
                                    }
                                    div { class: "badge badge-ghost badge-sm", "{type_label}" }
                                    p { class: "text-sm opacity-75 truncated", "ID: {node.id}" }
                                }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }
            }
        }
    }
}

#[component]
fn ContractWizard(on_close: EventHandler<()>, on_create: EventHandler<()>) -> Element {
    let app_state = use_context::<AppState>();
    let local_peer_id = app_state.local_peer_id.read().clone();
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    let mut contract_type = use_signal(|| "Payment".to_string());
    let mut title = use_signal(|| "".to_string());
    
    // Rent/Payment Fields
    let mut landlord_id = use_signal(|| "".to_string());
    let mut monthly_rent = use_signal(|| "".to_string());
    let mut payment_interval = use_signal(|| "Monthly".to_string());

    // Loan Fields
    let mut borrower_id = use_signal(|| "".to_string());
    let mut loan_amount = use_signal(|| "".to_string());
    let mut interest_rate = use_signal(|| "".to_string());
    let mut loan_duration = use_signal(|| "".to_string()); // in months
    let mut repayment_interval = use_signal(|| "Monthly".to_string());
    
    // Validation State
    let mut error_msg = use_signal(|| "".to_string());
    
    let cmd_tx_deploy = cmd_tx.clone();
    let handle_create = move |_| {
        error_msg.set("".to_string());
        
        // Validation
        let c_type = contract_type.read();
        if c_type.as_str() == "Payment" {
             if landlord_id.read().trim().is_empty() { error_msg.set("Provider Peer ID is required".into()); return; }
             if monthly_rent.read().trim().is_empty() { error_msg.set("Amount is required".into()); return; }
        } else if c_type.as_str() == "Loan" {
             if borrower_id.read().trim().is_empty() { error_msg.set("Borrower Peer ID is required".into()); return; }
             if loan_amount.read().trim().is_empty() { error_msg.set("Principal is required".into()); return; }
             if interest_rate.read().trim().is_empty() { error_msg.set("Interest Rate is required".into()); return; }
        }

        let title_val = if title.read().is_empty() {
             format!("{} Agreement", *c_type)
        } else {
             title.read().clone()
        };

        let params = match c_type.as_str() {
            "Payment" => {
                 // Recurring Payment
                 serde_json::json!({
                    "metadata": { "title": title_val, "type_label": "Payment" },
                     "parties": { "provider": *landlord_id.read(), "consumer": local_peer_id },
                     "payment_terms": {
                        "type": "recurring",
                        "amount": monthly_rent.read().parse::<f64>().unwrap_or(0.0),
                        "interval": *payment_interval.read(),
                        "currency": "Tokens"
                    }
                }).to_string()
            }
            "Loan" => {
                 serde_json::json!({
                    "metadata": { "title": title_val, "type_label": "Loan" },
                    "parties": { "provider": local_peer_id, "consumer": *borrower_id.read() },
                    "payment_terms": {
                        "type": "loan",
                        "principal": loan_amount.read().parse::<f64>().unwrap_or(0.0),
                        "interest_rate": interest_rate.read().parse::<f64>().unwrap_or(0.0),
                        "duration_months": loan_duration.read().parse::<u32>().unwrap_or(12),
                        "repayment_interval": *repayment_interval.read(),
                        "currency": "Tokens"
                    }
                 }).to_string()
            }
            _ => "{}".to_string()
        };
        
        let code = r#"
            // Standard Agreement
        "#.to_string();

        cmd_tx_deploy.send(AppCmd::DeployContract { code, init_params: params }); // error ignored in context
        on_create.call(());
    };

    rsx! {
        div { class: "card bg-base-100 shadow-2xl p-6 mb-6",
            div { class: "flex justify-between items-center mb-4",
                h2 { class: "text-xl font-bold", "New Agreement" }
                button { class: "btn btn-ghost btn-sm", onclick: move |_| on_close.call(()), "X" }
            }
            
            div { class: "flex flex-col gap-4",
                // Type Selection
                div { class: "form-control w-full",
                    label { class: "label", span { class: "label-text font-semibold", "Agreement Type" } }
                    select { 
                        class: "select select-bordered",
                        onchange: move |evt| contract_type.set(evt.value()),
                        option { value: "Payment", "Recurring Payment" }
                        option { value: "Loan", "Loan" }
                    }
                }
                
                // Title
                div { class: "form-control w-full",
                    label { class: "label", span { class: "label-text", "Agreement Title (Optional)" } }
                    input { 
                        class: "input input-bordered", 
                        placeholder: "e.g. Apartment Rent, Consulting Fee",
                        value: "{title}", 
                        oninput: move |e| title.set(e.value()) 
                    }
                }
                
                div { class: "divider", "Details" }
                
                // Payment Fields
                if *contract_type.read() == "Payment" {
                     div { class: "form-control",
                        label { class: "label", "Payee Peer ID (Provider) *" }
                        input { class: "input input-bordered", placeholder: "Paste peer ID here", value: "{landlord_id}", oninput: move |e| landlord_id.set(e.value()) }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        div { class: "form-control",
                            label { class: "label", "Amount (Tokens) *" }
                            input { class: "input input-bordered", type: "number", placeholder: "100", value: "{monthly_rent}", oninput: move |e| monthly_rent.set(e.value()) }
                        }
                        div { class: "form-control",
                            label { class: "label", "Payment Interval" }
                            select { 
                                class: "select select-bordered w-full",
                                onchange: move |evt| payment_interval.set(evt.value()),
                                option { value: "Monthly", "Monthly" }
                                option { value: "Weekly", "Weekly" }
                                option { value: "Daily", "Daily" }
                                option { value: "Yearly", "Yearly" }
                                option { value: "Once", "One-Time" }
                            }
                        }
                    }
                }
                
                // Loan Fields  
                if *contract_type.read() == "Loan" {
                     div { class: "form-control",
                        label { class: "label", "Borrower Peer ID *" }
                        input { class: "input input-bordered", placeholder: "Paste peer ID here", value: "{borrower_id}", oninput: move |e| borrower_id.set(e.value()) }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        div { class: "form-control",
                            label { class: "label", "Principal (Tokens) *" }
                            input { class: "input input-bordered", type: "number", placeholder: "1000", value: "{loan_amount}", oninput: move |e| loan_amount.set(e.value()) }
                        }
                        div { class: "form-control",
                            label { class: "label", "Interest Rate (%) *" }
                            input { class: "input input-bordered", type: "number", placeholder: "5", value: "{interest_rate}", oninput: move |e| interest_rate.set(e.value()) }
                        }
                    }
                    div { class: "grid grid-cols-2 gap-4",
                        div { class: "form-control",
                            label { class: "label", "Duration (Months)" }
                            input { class: "input input-bordered", type: "number", placeholder: "12", value: "{loan_duration}", oninput: move |e| loan_duration.set(e.value()) }
                        }
                        div { class: "form-control",
                            label { class: "label", "Repayment Interval" }
                            select { 
                                class: "select select-bordered w-full",
                                onchange: move |evt| repayment_interval.set(evt.value()),
                                option { value: "Monthly", "Monthly" }
                                option { value: "Weekly", "Weekly" }
                                option { value: "Bi-Weekly", "Bi-Weekly" }
                                option { value: "Quarterly", "Quarterly" }
                                option { value: "Lump Sum", "Lump Sum" }
                            }
                        }
                    }
                }
            }
              
            if !error_msg.read().is_empty() {
                div { class: "alert alert-error mt-4", span { "{error_msg}" } }
            }

            div { class: "mt-6 flex justify-end",
                button { class: "btn btn-success", onclick: handle_create, "Create Agreement" }
            }
        }
    }
}

#[component]
fn ContractDetail(contract_id: String, on_back: EventHandler<()>) -> Element {
    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<UnboundedSender<AppCmd>>();
    let history = app_state.active_contract_history.read();
    
    // Fetch history on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(use_reactive(&contract_id, move |cid| {
        cmd_tx_effect.send(AppCmd::FetchContractHistory { contract_id: cid });
    }));

    // Find the contract definition
    let contracts = app_state.contracts.read();
    let contract_node = contracts.iter().find(|n| n.id == contract_id);

    if contract_node.is_none() {
        return rsx! { div { "Contract not found" } };
    }
    let node = contract_node.unwrap();
    let params: serde_json::Value = if let DagPayload::Contract(c) = &node.payload {
        serde_json::from_str(&c.init_params).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    
    let metadata = &params["metadata"];
    let payment = &params["payment_terms"];
    let parties = &params["parties"];

    let title = metadata["title"].as_str().unwrap_or("Smart Agreement");
    let p_type = payment["type"].as_str().unwrap_or("unknown");
    
    // Unified Logic: Provider (Payee) vs Consumer (Payer)
    let provider = parties["provider"].as_str().unwrap_or("?");
    let consumer = parties["consumer"].as_str().unwrap_or("?");

    // State Calculations
    let mut total_paid = 0.0;
    let mut last_payment_date = "Never".to_string();

    for h_node in history.iter() {
         match &h_node.payload {
             DagPayload::Token(token) => {
                 if let Some(r) = &token.ref_cid {
                     if r == &contract_id {
                          total_paid += token.amount as f64;
                          if last_payment_date == "Never" {
                               last_payment_date = h_node.timestamp.format("%Y-%m-%d").to_string();
                          }
                     }
                 }
             }
             _ => {}
         }
    }

    let cid_pay = contract_id.clone();
    let cmd_tx_pay = cmd_tx.clone();
    
    // Calculate Payment Amount based on terms
    let pay_amount = if p_type == "hourly" {
         let rate = payment["rate"].as_f64().unwrap_or(0.0);
         (rate * 8.0) as u64 // Default 8 hrs
    } else if p_type == "recurring" {
         payment["amount"].as_u64().unwrap_or(0)
    } else if p_type == "loan" {
         let total = payment["principal"].as_f64().unwrap_or(0.0);
         (total / 10.0) as u64 // Default 10% chunk
    } else {
        0
    };

    let handle_payment = move |_| {
        let amount = pay_amount; // capture calculated amount
        let _ = cmd_tx_pay.send(AppCmd::PayContract {
             contract_id: cid_pay.clone(),
             amount,
        });
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            button { class: "btn btn-ghost w-24", onclick: move |_| on_back.call(()), "← Back" }
            
            div { class: "card bg-base-100 shadow-xl p-6",
                h2 { class: "text-2xl font-bold mb-4", "{title}" }
                
                div { class: "grid grid-cols-2 gap-4 text-sm mb-6",
                    div { class: "font-semibold", "Contract ID:" } div { class: "opacity-75 truncated", "{contract_id}" }
                    div { class: "font-semibold", "Provider (Payee):" } div { class: "opacity-75 truncated", "{provider}" }
                    div { class: "font-semibold", "Consumer (Payer):" } div { class: "opacity-75 truncated", "{consumer}" }

                    div { class: "divider col-span-2", "Terms" }
                    if p_type == "recurring" {
                         div { class: "font-semibold", "Type:" } div { "Recurring ({payment[\"interval\"].as_str().unwrap_or(\"Monthly\")})" }
                         div { class: "font-semibold", "Amount:" } div { "{payment[\"amount\"]} Tokens" }
                         div { class: "font-semibold", "Last Payment:" } div { "{last_payment_date}" }
                         div { class: "font-semibold", "Total Paid:" } div { "{total_paid}" }
                    } else if p_type == "loan" {
                         div { class: "font-semibold", "Type:" } div { "Loan" }
                         div { class: "font-semibold", "Principal:" } div { "{payment[\"principal\"]} Tokens" }
                         div { class: "font-semibold", "Interest:" } div { "{payment[\"interest_rate\"]}%" }
                         div { class: "font-semibold", "Repaid:" } div { "{total_paid}" }
                         div { class: "font-semibold", "Remaining:" } div { "{payment[\"principal\"].as_f64().unwrap_or(0.0) - total_paid}" }
                    } else {
                         // Fallback for distinct legacy types or custom
                         div { class: "col-span-2 opacity-50", "Type: {p_type}" }
                    }
                }

                div { class: "flex gap-2 justify-end",
                    if p_type == "recurring" {
                        button { class: "btn btn-primary", onclick: handle_payment, "Pay {payment[\"interval\"].as_str().unwrap_or(\"Payment\")} ({pay_amount})" }
                    } else if p_type == "loan" {
                        button { class: "btn btn-primary", onclick: handle_payment, "Repay ({pay_amount})" }
                    }
                }
            }

            // History Section
            div { class: "card bg-base-100 shadow-xl p-6",
                h3 { class: "font-bold text-lg mb-4", "Activity History" }
                div { class: "overflow-x-auto",
                    table { class: "table w-full",
                        thead { tr { th { "Date" } th { "Action" } th { "Details" } } }
                        tbody {
                            for item in history.iter() {
                                tr {
                                    {
                                        let time_fmt = item.timestamp.format("%Y-%m-%d %H:%M").to_string();
                                        rsx! { td { "{time_fmt}" } }
                                    }
                                    td { 
                                        match &item.payload {
                                            DagPayload::ContractCall(c) => rsx!{ span { class: "badge badge-info", "Call: {c.method}" } },
                                            DagPayload::Token(_) => rsx!{ span { class: "badge badge-success", "Payment" } },
                                            _ => rsx!{ span { "Event" } }
                                        }
                                    }
                                    td { 
                                        match &item.payload {
                                            DagPayload::ContractCall(c) => "{c.params}",
                                            DagPayload::Token(t) => "Amount: {t.amount}",
                                            _ => "ID: {item.id}" // fixed string interpolation
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
