use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::components::AppState;
use crate::components::home_page::HomeComponent;
use crate::components::geohash_page::GeohashComponent;
use crate::components::governance_page::GovernanceComponent;
use crate::components::marketplace_page::MarketplaceComponent;
use crate::components::messaging_page::MessagingComponent;
use crate::components::profile_page::ProfileComponent;
use crate::components::transparency_page::TransparencyComponent;
use crate::components::browser_page::BrowserComponent;
use crate::components::education_page::EducationComponent;
use crate::components::verification_page::VerificationPage;

/// SuperWebShell is the primary container for all modules in the SuperWeb-first architecture.
/// It renders content based on the current `sp://` URL in app_state.browser_url.
#[component]
pub fn SuperWebShell() -> Element {
    let mut app_state = use_context::<AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let current_url = app_state.browser_url.read().clone();
    
    // Parse the sp:// URL and route to appropriate module
    let content = match parse_superweb_url(&current_url) {
        SuperWebRoute::Home => rsx! { HomeComponent {} },
        SuperWebRoute::Local => rsx! { GeohashComponent {} },
        SuperWebRoute::Governance => rsx! { GovernanceComponent {} },
        SuperWebRoute::Market => rsx! { MarketplaceComponent {} },
        SuperWebRoute::Messages => rsx! { MessagingComponent {} },
        SuperWebRoute::Profile => rsx! { ProfileComponent {} },
        SuperWebRoute::Transparency => rsx! { TransparencyComponent {} },
        SuperWebRoute::Browser => rsx! { BrowserComponent {} },
        SuperWebRoute::Education => rsx! { EducationComponent {} },
        SuperWebRoute::Verify => rsx! { VerificationPage {} },
        SuperWebRoute::UserProfile(peer_id) => rsx! { 
            crate::components::profile_page::UserProfileComponent { peer_id: peer_id } 
        },
        SuperWebRoute::WebContent(url) => {
            // Fetch and display web content
            let cmd_tx = cmd_tx.clone();
            let url_clone = url.clone();
            use_effect(move || {
                let _ = cmd_tx.send(AppCmd::FetchWebPage { url: url_clone.clone() });
            });
            
            let web_content = app_state.web_content.read().clone();
            rsx! {
                div { class: "page-container py-8 animate-fade-in",
                    div { class: "panel",
                        div { class: "panel-header",
                            h2 { class: "panel-title", "{url}" }
                        }
                        if let Some(html_content) = web_content {
                            div { 
                                class: "prose prose-invert max-w-none p-4",
                                dangerous_inner_html: "{html_content}" 
                            }
                        } else {
                            div { class: "prose prose-invert max-w-none p-4",
                                div { class: "empty-state",
                                    div { class: "empty-state-icon", "🌐" }
                                    p { class: "empty-state-title", "Loading..." }
                                }
                            }
                        }
                    }
                }
            }
        },
        SuperWebRoute::Welcome => {
            // Fetch all web pages for wiki directory
            let cmd_tx = cmd_tx.clone();
            use_effect(move || {
                let _ = cmd_tx.send(AppCmd::FetchAllWebPages);
            });
            
            let all_pages = app_state.all_web_pages.read();
            
            // Define pinned system pages
            let pinned_pages: Vec<(&str, &str, &str, &str)> = vec![
                ("Home", "🏠", "sp://home.super", "Social feed"),
                ("Local", "📍", "sp://local.super", "Your community"),
                ("Govern", "🏛", "sp://gov.super", "Democracy"),
                ("Market", "💰", "sp://market.super", "Trade"),
                ("Messages", "💬", "sp://messages.super", "Chat"),
                ("Profile", "👤", "sp://profile.super", "Identity"),
                ("Education", "🎓", "sp://edu.super", "Learn"),
                ("Publish", "✏️", "sp://browser.super", "Create pages"),
                ("Verify", "✅", "sp://verify.super", "Accept users"),
            ];
            
            let total_pages = pinned_pages.len() + all_pages.len();
            
            rsx! {
                div { class: "page-container py-8 animate-fade-in",
                    // Header
                    div { class: "text-center mb-8",
                        h1 { class: "text-4xl font-bold bg-gradient-to-r from-[var(--primary)] to-[var(--accent)] bg-clip-text text-transparent mb-2",
                            "🌐 SuperWeb"
                        }
                        p { class: "text-[var(--text-secondary)] text-lg",
                            "The decentralized web, powered by the people."
                        }
                        p { class: "text-sm text-[var(--text-muted)] mt-2",
                            "{total_pages} pages available"
                        }
                    }
                    
                    // Unified list: pinned first, then community pages
                    div { class: "space-y-3",
                        // Pinned pages (system apps)
                        for (title, icon, url, desc) in pinned_pages.iter() {
                            PageRow { 
                                title: title.to_string(), 
                                icon: Some(icon.to_string()), 
                                url: url.to_string(),
                                description: desc.to_string(),
                                pinned: true,
                            }
                        }
                        
                        // Community pages
                        for page in all_pages.iter() {
                            if let crate::backend::dag::DagPayload::Web(ref web) = page.payload {
                                {
                                    let url = web.url.clone();
                                    let title = web.title.clone();
                                    let description = web.description.clone();
                                    
                                    rsx! {
                                        PageRow { 
                                            key: "{page.id}",
                                            title: title, 
                                            icon: None, 
                                            url: url,
                                            description: description,
                                            pinned: false,
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Create page CTA if empty community
                    if all_pages.is_empty() {
                        div { class: "mt-8 text-center",
                            p { class: "text-[var(--text-secondary)] mb-4", 
                                "No community pages yet. Be the first to create one!" 
                            }
                            button { 
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    app_state.browser_url.set("sp://browser.super".to_string());
                                },
                                "✏️ Create a Page"
                            }
                        }
                    }
                }
            }
        },
    };

    rsx! {
        div { class: "superweb-shell flex flex-col min-h-screen",
            // SuperWeb Address Bar
            SuperWebAddressBar {}
            
            // Main content area
            div { class: "flex-1",
                {content}
            }
        }
    }
}

/// URL bar for SuperWeb navigation
#[component]
fn SuperWebAddressBar() -> Element {
    let mut app_state = use_context::<AppState>();
    let mut url_input = use_signal(|| app_state.browser_url.read().clone());
    
    // Sync input with browser_url changes
    use_effect(move || {
        url_input.set(app_state.browser_url.read().clone());
    });

    let on_navigate = move |_| {
        let url = url_input();
        if !url.is_empty() {
            app_state.browser_url.set(url);
        }
    };

    rsx! {
        div { class: "superweb-address-bar bg-[var(--bg-secondary)] border-b border-[var(--border-subtle)] px-4 py-2",
            div { class: "page-container flex items-center gap-2",
                // Back button
                button {
                    class: "btn btn-ghost btn-sm",
                    onclick: move |_| {
                        // Simple back - go to welcome
                        app_state.browser_url.set("sp://welcome".to_string());
                    },
                    "←"
                }
                
                // URL input
                div { class: "flex-1 flex gap-2",
                    input {
                        class: "input flex-1 text-sm font-mono",
                        placeholder: "sp://...",
                        value: "{url_input}",
                        oninput: move |e| url_input.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let url = url_input();
                                if !url.is_empty() {
                                    app_state.browser_url.set(url);
                                }
                            }
                        }
                    }
                    button {
                        class: "btn btn-primary btn-sm",
                        onclick: on_navigate,
                        "Go"
                    }
                }
            }
        }
    }
}

/// Card component for module selection on welcome page
#[component]
fn ModuleCard(title: String, icon: String, url: String, description: String) -> Element {
    let mut app_state = use_context::<AppState>();
    
    rsx! {
        div { 
            class: "card cursor-pointer hover:bg-[var(--bg-elevated)] transition-all hover:scale-105 p-4 text-center",
            onclick: move |_| {
                app_state.browser_url.set(url.clone());
            },
            div { class: "text-3xl mb-2", "{icon}" }
            h3 { class: "font-bold text-[var(--text-primary)]", "{title}" }
            p { class: "text-xs text-[var(--text-secondary)] mt-1", "{description}" }
        }
    }
}

/// Inline page row for list display - uses card styling
#[component]
fn PageRow(
    title: String, 
    icon: Option<String>, 
    url: String, 
    description: String,
    pinned: bool,
) -> Element {
    let mut app_state = use_context::<AppState>();
    
    rsx! {
        button { 
            class: "card w-full flex items-center gap-4 text-left cursor-pointer",
            onclick: move |_| {
                app_state.browser_url.set(url.clone());
            },
            // Pin indicator
            if pinned {
                span { class: "text-base flex-shrink-0", "📌" }
            }
            
            // Icon
            if let Some(ref ico) = icon {
                span { class: "text-2xl flex-shrink-0", "{ico}" }
            } else {
                span { class: "text-2xl flex-shrink-0", "📄" }
            }
            
            // Title + Description
            div { class: "flex-1 min-w-0",
                span { class: "font-semibold text-[var(--text-primary)]", "{title}" }
                if !description.is_empty() {
                    span { class: "text-sm text-[var(--text-secondary)] ml-2", "— {description}" }
                }
            }
            
            // Arrow
            span { class: "text-lg text-[var(--text-muted)] flex-shrink-0", "→" }
        }
    }
}




/// Enum representing internal SuperWeb routes
enum SuperWebRoute {
    Home,
    Local,
    Governance,
    Market,
    Messages,
    Profile,
    Transparency,
    Browser,
    Education,
    Verify,
    UserProfile(String),
    WebContent(String),
    Welcome,
}

/// Parse sp:// URL into a route
fn parse_superweb_url(url: &str) -> SuperWebRoute {
    let url = url.trim();
    
    match url {
        "sp://home.super" | "sp://home.super/" => SuperWebRoute::Home,
        "sp://local.super" | "sp://local.super/" => SuperWebRoute::Local,
        "sp://gov.super" | "sp://gov.super/" => SuperWebRoute::Governance,
        "sp://market.super" | "sp://market.super/" => SuperWebRoute::Market,
        "sp://messages.super" | "sp://messages.super/" => SuperWebRoute::Messages,
        "sp://profile.super" | "sp://profile.super/" => SuperWebRoute::Profile,
        "sp://transparency.super" | "sp://transparency.super/" => SuperWebRoute::Transparency,
        "sp://browser.super" | "sp://browser.super/" | "sp://files.super" | "sp://files.super/" => SuperWebRoute::Browser,
        "sp://edu.super" | "sp://edu.super/" | "sp://learn.super" | "sp://learn.super/" => SuperWebRoute::Education,
        "sp://verify.super" | "sp://verify.super/" => SuperWebRoute::Verify,
        "sp://welcome" | "sp://welcome/" | "sp://super.app" | "sp://super.app/" => SuperWebRoute::Welcome,
        _ => {
            // Check for user profile: sp://profile.super/peer_id
            if url.starts_with("sp://profile.super/") {
                let peer_id = url.strip_prefix("sp://profile.super/").unwrap_or("").to_string();
                if !peer_id.is_empty() && peer_id != "/" {
                    return SuperWebRoute::UserProfile(peer_id.trim_end_matches('/').to_string());
                }
            }
            
            // Any other sp:// URL is web content
            if url.starts_with("sp://") {
                SuperWebRoute::WebContent(url.to_string())
            } else {
                SuperWebRoute::Welcome
            }
        }
    }
}
