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
    let mut category = use_signal(|| "".to_string());
    let mut filter_certified_only = use_signal(|| false);
    let mut filter_local = use_signal(|| false);
    let mut tag_location = use_signal(|| true); // Default to tagging with location

    // Fetch listings on mount
    let cmd_tx_effect = cmd_tx.clone();
    let current_geohash = app_state.geohash.read().clone();
    use_effect(move || {
        if filter_local() && current_geohash != "Global" {
            let prefix = current_geohash.chars().take(4).collect::<String>();
            let _ = cmd_tx_effect.send(AppCmd::FetchLocalListings { geohash_prefix: prefix });
        } else {
            let _ = cmd_tx_effect.send(AppCmd::FetchListings);
        }
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
                    category: if category().is_empty() { None } else { Some(category()) },
                    geohash: if tag_location() && *app_state.geohash.read() != "Global" {
                        Some(app_state.geohash.read().clone())
                    } else {
                        None
                    },
                });
                title.set(String::new());
                description.set(String::new());
                price.set(String::new());
                category.set("".to_string());
                tag_location.set(true);
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

    // Use absolute path to Route
    use crate::Route;

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
                            onclick: move |_| {
                                let mut app_state = use_context::<crate::components::AppState>();
                                app_state.browser_url.set("sp://contracts.super".to_string());
                            },
                            "Contracts"
                        }
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
            div { class: "panel mb-6 flex flex-col md:flex-row gap-2",
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
                div { class: "flex items-center gap-2 px-2",
                    input {
                        "type": "checkbox",
                        class: "checkbox",
                        checked: "{filter_certified_only}",
                        onchange: move |e| filter_certified_only.set(e.checked())
                    }
                    span { "Certified Providers Only" }
                }
                div { class: "flex items-center gap-2 px-2 border-l border-[var(--border-color)]",
                    input {
                        "type": "checkbox",
                        class: "checkbox",
                        checked: "{filter_local}",
                        onchange: move |e| filter_local.set(e.checked())
                    }
                    div { class: "flex flex-col",
                        span { class: "text-sm font-medium", "Local Market Only" }
                        if filter_local() {
                            span { class: "text-[8px] text-[var(--text-muted)]", 
                                if *app_state.geohash.read() == "Global" { "Requires Location" } else { "Showing {app_state.geohash.read().chars().take(4).collect::<String>()}..." }
                            }
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
                    div { class: "form-group border-t border-[var(--border-color)] pt-4 flex items-center gap-2",
                        input {
                            "type": "checkbox",
                            class: "checkbox",
                            checked: "{tag_location}",
                            onchange: move |e| tag_location.set(e.checked())
                        }
                        div {
                            label { class: "text-sm font-medium", "Tag with my current location" }
                            p { class: "text-xs text-[var(--text-muted)]", "Allow nearby users to find your listing easily." }
                        }
                    }
                    button { class: "btn btn-primary mt-2", onclick: on_create, "Create Listing" }
                }
            }

            // Listings grid
            // Listings grid
            {
                let all_listings = app_state.listings.read();
                let local_listings = app_state.local_listings.read();
                let current_listings = if filter_local() { &*local_listings } else { &*all_listings };

                if current_listings.is_empty() {
                    rsx! {
                        div { class: "empty-state py-12",
                            div { class: "empty-state-icon", "🏪" }
                            p { class: "empty-state-title", "No listings found" }
                            p { class: "empty-state-text", "Try a different search or create a listing." }
                        }
                    }
                } else {
                    rsx! {
                        div { class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6",
                            for node in current_listings.iter() {
                                if let DagPayload::Listing(ListingPayload { title: item_title, description: item_desc, price: item_price, category: item_cat, geohash: item_gh, .. }) = &node.payload {
                                    if filter_certified_only() && item_cat.is_none() {
                                        // Skip
                                    } else {
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
                                                    div { class: "flex justify-between items-start mb-2",
                                                        div {
                                                            if let Some(cat) = item_cat {
                                                                 span { class: "badge badge-primary mb-1 inline-flex items-center gap-1 text-[10px]", 
                                                                    span { "✓" }
                                                                    "{cat}"
                                                                 }
                                                            }
                                                            h3 { class: "text-lg font-semibold", "{item_title}" }
                                                        }
                                                        if let Some(gh) = item_gh {
                                                            span { class: "text-[10px] bg-[var(--bg-secondary)] px-2 py-0.5 rounded-full border border-[var(--border-color)]", 
                                                                "📍 {gh.chars().take(4).collect::<String>()}" 
                                                            }
                                                        }
                                                    }

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
                                                            } else {
                                                                button {
                                                                    class: "btn btn-primary btn-sm",
                                                                    onclick: move |_| {
                                                                        let _ = cmd_tx_buy.send(AppCmd::BuyListing { listing_id: listing_id.clone() });
                                                                    },
                                                                    "Buy Now"
                                                                }
                                                            }
                                                        }
                                                    }
                                                    
                                                    div { class: "flex justify-between items-center mt-3",
                                                        p { class: "text-xs text-[var(--text-muted)]", 
                                                            "Listed by: {author.get(0..12).unwrap_or(&author)}..."
                                                        }
                                                        if !is_my_listing {
                                                            button {
                                                                class: "text-[10px] text-[var(--primary)] hover:underline",
                                                                onclick: {
                                                                    let peer_id = author.clone();
                                                                    move |_| {
                                                                        let mut app_state = use_context::<crate::components::AppState>();
                                                                        app_state.browser_url.set(format!("sp://profile.super/{}", peer_id));
                                                                    }
                                                                },
                                                                "View Seller"
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
    }
}
