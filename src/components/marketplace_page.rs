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
    let mut search_query = use_signal(|| String::new());

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

    let cmd_tx_search = cmd_tx.clone();
    let on_search = move |_| {
        if search_query().is_empty() {
            let _ = cmd_tx_search.send(AppCmd::FetchListings);
        } else {
            let _ = cmd_tx_search.send(AppCmd::SearchListings { query: search_query() });
        }
    };

    let my_id = app_state.local_peer_id.read().clone();

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

            // Search Bar
            div { class: "panel mb-6 flex gap-2",
                input {
                    class: "input flex-1",
                    placeholder: "Search listings...",
                    value: "{search_query}",
                    oninput: move |e| search_query.set(e.value()),
                    onkeydown: move |e| {
                        if e.key() == Key::Enter {
                            on_search(());
                        }
                    }
                }
                {
                    let on_search_click = on_search.clone();
                    rsx! {
                        button { class: "btn btn-secondary", onclick: move |_| on_search_click(()), "Search" }
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
                    p { class: "empty-state-title", "No listings found" }
                    p { class: "empty-state-text", "Try a different search or create a listing." }
                }
            } else {
                div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                    for node in app_state.listings.read().iter() {
                        if let DagPayload::Listing(ListingPayload { title: item_title, description: item_desc, price: item_price, .. }) = &node.payload {
                            {
                                let author = node.author.clone();
                                let is_my_listing = author == my_id;
                                let listing_id = node.id.clone();
                                let cmd_tx_buy = cmd_tx.clone();

                                let cmd_tx_status = cmd_tx.clone();
                                let cmd_tx_cancel = cmd_tx.clone();
                                let listing_id_cancel = listing_id.clone();
                                
                                rsx! {
                                    div {
                                        class: "panel flex flex-col h-full",
                                        key: "{node.id}",
                                        
                                        div { class: "p-4 flex-1",
                                            h3 { class: "text-lg font-semibold mb-2", "{item_title}" }
                                            p { class: "text-[var(--text-secondary)] text-sm mb-4 line-clamp-2", "{item_desc}" }
                                            
                                            div { class: "flex justify-between items-center pt-4 border-t border-[var(--border-color)]",
                                                div {
                                                    p { class: "text-2xl font-bold text-[var(--primary)]", "{item_price} SUPER" }
                                                }
                                                div { class: "flex gap-2",
                                                    if is_my_listing {
                                                         button {
                                                            class: "btn btn-sm btn-secondary",
                                                            onclick: move |_| {
                                                                let _ = cmd_tx_status.send(AppCmd::UpdateListingStatus { listing_id: listing_id.clone(), status: crate::backend::dag::ListingStatus::Sold });
                                                            },
                                                            "Mark Sold"
                                                        }
                                                        button {
                                                            class: "btn btn-sm btn-danger ml-1", // Assuming btn-danger exists or falls back
                                                             onclick: move |_| {
                                                                let _ = cmd_tx_cancel.send(AppCmd::UpdateListingStatus { listing_id: listing_id_cancel.clone(), status: crate::backend::dag::ListingStatus::Cancelled });
                                                            },
                                                            "Cancel"
                                                        }
                                                    } else {
                                                        button {
                                                            class: "btn btn-primary btn-sm",
                                                            onclick: move |_| {
                                                                let _ = cmd_tx_buy.send(AppCmd::BuyListing { listing_id: listing_id.clone() });
                                                            },
                                                            "Buy Now"
                                                        }
                                                        Link {
                                                            to: crate::Route::UserProfileComponent { peer_id: author.clone() },
                                                            class: "btn btn-secondary btn-sm",
                                                            "Seller"
                                                        }
                                                    }
                                                }
                                            }
                                            
                                            p { class: "text-xs text-[var(--text-muted)] mt-3", 
                                                "Listed by: {author.get(0..12).unwrap_or(&author)}..."
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
