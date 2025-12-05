use dioxus::prelude::*;
use crate::backend::{AppCmd, AppEvent};
use crate::components::AppState;
use hex;

#[component]
pub fn BrowserComponent() -> Element {
    let mut app_state = use_context::<AppState>();
    let mut url_input = use_signal(|| "sp://".to_string());
    let mut is_loading = use_signal(|| false);
    let mut show_publish_form = use_signal(|| false);
    
    // Publish form state
    let mut publish_url = use_signal(|| "sp://".to_string());
    let mut publish_title = use_signal(|| "".to_string());
    let mut publish_content = use_signal(|| "<h1>Hello World</h1>".to_string());
    let mut publish_desc = use_signal(|| "".to_string());
    let mut publish_tags = use_signal(|| "".to_string());


    let current_content = use_memo(move || {
        app_state.web_content.read().clone().unwrap_or_else(|| "<h1>Welcome to SuperWeb!</h1><p>Enter a URL (sp://...) to browse or search for content.</p>".to_string())
    });

    let mut submit_action = move || {
        let input = url_input();
        is_loading.set(true);
        if let Some(cmd_tx) = use_context::<Option<tokio::sync::mpsc::UnboundedSender<AppCmd>>>() {
            if input.starts_with("sp://") {
                 let _ = cmd_tx.send(AppCmd::FetchWebPage { url: input.clone() });
                 // Clear search results when navigating
                 if app_state.web_search_results.read().len() > 0 {
                     app_state.web_search_results.write().clear();
                 }
            } else {
                 // Treat as search query
                 let _ = cmd_tx.send(AppCmd::SearchWeb { query: input.clone() });
            }
        }
    };

    let on_publish = move |_| {
        if let Some(cmd_tx) = use_context::<Option<tokio::sync::mpsc::UnboundedSender<AppCmd>>>() {
            let tags_vec: Vec<String> = publish_tags().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            
            let _ = cmd_tx.send(AppCmd::PublishWebPage { 
                url: publish_url(), 
                title: publish_title(),
                content: publish_content(),
                description: publish_desc(),
                tags: tags_vec
            });
            show_publish_form.set(false);
            // Navigate to the new page
            url_input.set(publish_url());
            let _ = cmd_tx.send(AppCmd::FetchWebPage { url: publish_url() });
        }
    };

    rsx! {
        div { class: "page-container py-8 animate-fade-in flex flex-col min-h-[calc(100vh-64px)]",
            
            // Header
            div { class: "page-header",
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

            // Publish Form
            if show_publish_form() {
                div { class: "panel mb-6",
                    h2 { class: "text-lg font-semibold mb-4", "Publish New Page" }
                    div { class: "grid gap-4",
                        div { class: "form-group",
                            label { class: "form-label", "URL (sp://...)" }
                            input { class: "input", value: "{publish_url}", oninput: move |e| publish_url.set(e.value()) }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Title" }
                            input { class: "input", value: "{publish_title}", oninput: move |e| publish_title.set(e.value()) }
                        }
                         div { class: "form-group",
                            label { class: "form-label", "Description" }
                            input { class: "input", value: "{publish_desc}", oninput: move |e| publish_desc.set(e.value()) }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Tags (comma separated)" }
                            input { class: "input", value: "{publish_tags}", oninput: move |e| publish_tags.set(e.value()) }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "Content (HTML or WASM Hex)" }
                            textarea { 
                                class: "input min-h-[150px]", 
                                value: "{publish_content}", 
                                oninput: move |e| publish_content.set(e.value()) 
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", "OR Upload Dynamic Site (.wasm)" }
                            input {
                                class: "input",
                                r#type: "file",
                                accept: ".wasm",
                                onchange: move |evt| {
                                    let files = evt.files();
                                    if !files.is_empty() {
                                        spawn(async move {
                                            for file in files {
                                                 if file.name().ends_with(".wasm") {
                                                     if let Ok(bytes) = file.read_bytes().await {
                                                         let hex_wasm = hex::encode(bytes);
                                                         publish_content.set(hex_wasm);
                                                         publish_title.set(file.name());
                                                     }
                                                 }
                                            }
                                        });
                                    }
                                }
                            }
                            p { class: "text-xs text-[var(--text-secondary)] mt-1", "Uploading a .wasm file will replace the specific content above with the hex-encoded binary." }
                        }

                        button { class: "btn btn-primary", onclick: on_publish, "Publish" }
                    }
                }
            }

            // Address bar / Search
            div { class: "panel mb-6",
                div { class: "flex gap-4 items-center",
                    div { class: "flex-1",
                        input {
                            class: "input",
                            value: "{url_input}",
                            placeholder: "Enter URL (sp://) or Search Query...",
                            oninput: move |e| url_input.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    submit_action();
                                }
                            }
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| submit_action(),
                        "Go / Search"
                    }
                }
            }

            // Search Results
            if !app_state.web_search_results.read().is_empty() {
                div { class: "mb-6",
                    h3 { class: "text-lg font-semibold mb-3", "Search Results" }
                    div { class: "grid gap-4",
                        {
                            let results = app_state.web_search_results.read().clone();
                            rsx! {
                                for node in results.iter() {
                                    if let crate::backend::dag::DagPayload::Web(web) = &node.payload {
                                        {
                                            let url = web.url.clone();
                                            rsx! {
                                                div { 
                                                    class: "panel cursor-pointer hover:border-[var(--primary)] transition-colors",
                                                    onclick: move |_| {
                                                        url_input.set(url.clone());
                                                        submit_action();
                                                    },
                                                    div { class: "flex justify-between items-start",
                                                         h4 { class: "font-semibold text-[var(--primary)]", "{web.title}" }
                                                         span { class: "text-xs text-[var(--text-muted)]", "{web.url}" }
                                                    }
                                                    p { class: "text-sm text-[var(--text-secondary)] mt-1", "{web.description}" }
                                                    div { class: "flex gap-2 mt-2",
                                                        for tag in &web.tags {
                                                            span { class: "px-2 py-0.5 bg-[var(--bg-secondary)] rounded text-xs", "#{tag}" }
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

            // Content area
            if app_state.web_search_results.read().is_empty() {
                div {
                    class: "panel flex-1 overflow-y-auto",
                    style: "background: #ffffff; color: #000000; min-height: 400px; padding: 20px;",
                    dangerous_inner_html: "{current_content}"
                }
            }
        }
    }
}
