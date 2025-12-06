use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::components::AppState;
use crate::Route;
use hex;

#[component]
pub fn BrowserComponent() -> Element {
    let mut app_state = use_context::<AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let nav = use_navigator();
    let mut url_input = use_signal(|| "sp://".to_string());
    let mut is_loading = use_signal(|| false);
    let mut show_publish_form = use_signal(|| false);
    
    // Publish form state
    let publish_url = use_signal(|| "sp://".to_string());
    let publish_title = use_signal(|| "".to_string());
    let publish_content = use_signal(|| "<h1>Hello World</h1>".to_string());
    let publish_desc = use_signal(|| "".to_string());
    let publish_tags = use_signal(|| "".to_string());


    let _current_content = use_memo(move || {
        app_state.web_content.read().clone().unwrap_or_else(|| "<h1>Welcome to SuperWeb!</h1><p>Enter a URL (sp://...) to browse or search for content.</p><p style='margin-top:20px;'>Try <strong>sp://gov.super</strong> to access the Governance Portal.</p>".to_string())
    });

    let nav_submit = nav.clone();
    let cmd_tx_submit = cmd_tx.clone();
    let _submit_action = move || {
        let input = url_input();
        is_loading.set(true);
        
        // Special handling for sp://gov.super - navigate to the governance page
        if input == "sp://gov.super" || input == "sp://gov.super/" {
            nav_submit.push(Route::GovernanceComponent {});
            is_loading.set(false);
            return;
        }

        // Special handling for News and Community
        if input == "sp://news.super" || input == "sp://news.super/" {
            let _ = cmd_tx_submit.send(AppCmd::FetchPosts);
            is_loading.set(false);
            return;
        }
        
         if input == "sp://community.super" || input == "sp://community.super/" {
            // Already have peers in app_state, maybe fetch candidates too if needed
             is_loading.set(false);
            return;
        }
        
        if input.starts_with("sp://") {
                 let _ = cmd_tx_submit.send(AppCmd::FetchWebPage { url: input.clone() });
                 // Clear search results when navigating
                 if app_state.web_search_results.read().len() > 0 {
                     app_state.web_search_results.write().clear();
                 }
        } else {
                 // Treat as search query
                 let _ = cmd_tx_submit.send(AppCmd::SearchWeb { query: input.clone() });
        }
    };

    let cmd_tx_publish = cmd_tx.clone();
    let _on_publish = move |_: Event<MouseData>| {
            let tags_vec: Vec<String> = publish_tags().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            
            let _ = cmd_tx_publish.send(AppCmd::PublishWebPage { 
                url: publish_url(), 
                title: publish_title(),
                content: publish_content(),
                description: publish_desc(),
                tags: tags_vec
            });
            show_publish_form.set(false);
            // Navigate to the new page
            url_input.set(publish_url());
            let _ = cmd_tx_publish.send(AppCmd::FetchWebPage { url: publish_url() });
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in flex flex-col min-h-[calc(100vh-64px)]",
            
            // Header
            div { class: "page-header",
            // ... (keep existing header code if possible, but for replace_file_content I need to provide full block if I selected a large range)
            // To be safe I will rewrite the components.
               div { class: "flex justify-between items-center",
                    div {
                        h1 { class: "page-title", "SuperWeb Browser" }
                        p { class: "text-[var(--text-secondary)]", "Decentralized Web & Search" }
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| show_publish_form.set(!show_publish_form()),
                        if show_publish_form() { "Cancel" } else { "Publish Page" }
                    }
                }
            }

            // Publish Form (omitted for brevity in prompt, but must include in actual tool call if replacing)
            // ... I will skip full rewrite of Publish Form if I can target smaller chunks, but I selected lines 8-260? No, I need to be careful.
            // I'll target the whole file content to be safely consistent, or use multi-replace.
            // Let's use multi-replace to target specific blocks. 
            // WAIT - I am inside the tool call definition. I should use multi_replace for better precision.
            // Cancelling this tool call and switching to multi_replace.
        }
    }
}

