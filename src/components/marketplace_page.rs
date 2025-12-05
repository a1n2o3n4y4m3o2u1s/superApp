use dioxus::prelude::*;
use crate::backend::{AppCmd, dag::{DagPayload, ListingPayload}};

#[component]
pub fn MarketplaceComponent() -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();

    let mut title = use_signal(|| String::new());
    let mut description = use_signal(|| String::new());
    let mut price = use_signal(|| String::new());
    let mut show_create_form = use_signal(|| false);

    // Fetch listings on mount
    let cmd_tx_effect = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_effect.send(AppCmd::FetchListings);
    });

    let cmd_tx_create = cmd_tx.clone();
    let on_create = move |_| {
        if let Ok(price_val) = price().parse::<u64>() {
            if !title().is_empty() {
                let _ = cmd_tx_create.send(AppCmd::CreateListing {
                    title: title(),
                    description: description(),
                    price: price_val,
                    image_cid: None,
                });
                title.set(String::new());
                description.set(String::new());
                price.set(String::new());
                show_create_form.set(false);
                // Refresh listings
                let _ = cmd_tx_create.send(AppCmd::FetchListings);
            }
        }
    };

    let cmd_tx_refresh = cmd_tx.clone();
    let on_refresh = move |_| {
        let _ = cmd_tx_refresh.send(AppCmd::FetchListings);
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Header
            div { class: "page-header",
                div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "page-title", "Marketplace" }
                        p { class: "text-[var(--text-secondary)] mt-1", "Buy and sell with SUPER tokens" }
                    }
                    div { class: "flex gap-2",
                        button { 
                            class: "btn btn-secondary", 
                            onclick: on_refresh,
                            "Refresh"
                        }
                        button { 
                            class: "btn btn-primary", 
                            onclick: move |_| show_create_form.set(!show_create_form()),
                            if show_create_form() { "Cancel" } else { "+ New Listing" }
                        }
                    }
                }
            }

            // Create listing form
            if show_create_form() {
                div { class: "panel mb-6",
                    div { class: "panel-header",
                        h2 { class: "panel-title", "Create Listing" }
                    }
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                        div { class: "form-group",
                            label { class: "form-label", "Title" }
                            input {
                                class: "input",
                                placeholder: "What are you selling?",
                                value: "{title}",
                                oninput: move |e| title.set(e.value())
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Price (SUPER)" }
                            input {
                                class: "input",
                                r#type: "number",
                                placeholder: "0",
                                value: "{price}",
                                oninput: move |e| price.set(e.value())
                            }
                        }
                    }
                    div { class: "form-group",
                        label { class: "form-label", "Description" }
                        textarea {
                            class: "input",
                            style: "min-height: 80px; resize: none;",
                            placeholder: "Describe your item...",
                            value: "{description}",
                            oninput: move |e| description.set(e.value())
                        }
                    }
                    button { class: "btn btn-primary", onclick: on_create, "Create Listing" }
                }
            }

            // Listings grid
            if app_state.listings.read().is_empty() {
                div { class: "empty-state py-12",
                    div { class: "empty-state-icon", "🏪" }
                    p { class: "empty-state-title", "No listings yet" }
                    p { class: "empty-state-text", "Be the first to list something for sale!" }
                }
            } else {
                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                    for node in app_state.listings.read().iter() {
                        if let DagPayload::Listing(ListingPayload { title: item_title, description: item_desc, price: item_price, .. }) = &node.payload {
                            div {
                                class: "panel",
                                key: "{node.id}",
                                
                                div { class: "p-4",
                                    h3 { class: "text-lg font-semibold mb-2", "{item_title}" }
                                    p { class: "text-[var(--text-secondary)] text-sm mb-4 line-clamp-2", "{item_desc}" }
                                    
                                    div { class: "flex justify-between items-center",
                                        div {
                                            p { class: "text-2xl font-bold", "{item_price} SUPER" }
                                        }
                                        Link {
                                            to: crate::Route::UserProfileComponent { peer_id: node.author.clone() },
                                            class: "btn btn-secondary btn-sm",
                                            "Contact Seller"
                                        }
                                    }
                                    
                                    p { class: "text-xs text-[var(--text-muted)] mt-3", 
                                        "Listed by: {node.author.get(0..12).unwrap_or(&node.author)}..."
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
