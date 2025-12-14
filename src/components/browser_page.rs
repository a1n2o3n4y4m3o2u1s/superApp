use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::components::AppState;
use crate::Route;

#[component]
pub fn BrowserComponent() -> Element {
    let app_state = use_context::<AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let nav = use_navigator();
    
    let mut url_input = use_signal(|| "sp://".to_string());
    let mut is_loading = use_signal(|| false);
    let mut show_publish_form = use_signal(|| false);
    let mut search_mode = use_signal(|| "web"); // "web" or "file"
    
    // Publish form state
    let mut publish_url = use_signal(|| "sp://".to_string());
    let mut publish_title = use_signal(|| "".to_string());
    let mut publish_content = use_signal(|| "<h1>Hello World</h1>".to_string());
    let mut publish_desc = use_signal(|| "".to_string());
    let mut publish_tags = use_signal(|| "".to_string());
    
    // Report form state
    let mut show_report_modal = use_signal(|| false);
    let mut report_reason = use_signal(|| "Spam".to_string());
    let mut report_details = use_signal(|| "".to_string());

    let _current_content = use_memo(move || {
        app_state.web_content.read().clone().unwrap_or_else(|| "<h1>Welcome to SuperWeb!</h1><p>Enter a URL (sp://...) to browse or search for content.</p><p style='margin-top:20px;'>Try <strong>sp://gov.super</strong> to access the Governance Portal.</p>".to_string())
    });

    let nav_submit = nav.clone();
    let cmd_tx_submit = cmd_tx.clone();
    let submit_action = move || {
        let input = url_input();
        is_loading.set(true);
        
        // Special URLs - gov.super is now handled internally by SuperWebShell
        if input == "sp://gov.super" || input == "sp://gov.super/" {
            // This URL is handled by the SuperWebShell, just set loading to false
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
                     let mut results = app_state.web_search_results;
                     results.write().clear();
                 }
        } else {
                 // Treat as search query
                 if search_mode() == "file" {
                     let _ = cmd_tx_submit.send(AppCmd::SearchFiles { query: input.clone() });
                 } else {
                     let _ = cmd_tx_submit.send(AppCmd::SearchWeb { query: input.clone() });
                 }
        }
    };
    
    let cmd_tx_publish = cmd_tx.clone();
    let on_publish = move |_| {
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

    let cmd_tx_report = cmd_tx.clone();
    let on_report_submit = move |_| {
        // We report the URL as the ID for now, as we might not have the CID easily accessible in UI state without more plumbing.
        // Ideally AppState.web_content should store (cid, content).
        // For MVP, passing URL is "okay" if backend can resolve it, but ReportPayload expects CID.
        // Let's rely on the user being on the page and maybe we can find the CID from the history or look it up?
        // Actually, we should probably report the URL *or* the CID.
        // Since ReportPayload has `target_id`, we should put the URL if we don't have CID.
        // Or better: Use the URL as the target_id and let backend/moderators handle it.
        let target = url_input(); 
        
        let _ = cmd_tx_report.send(AppCmd::ReportContent {
            target_id: target,
            reason: report_reason(),
            details: report_details()
        });
        show_report_modal.set(false);
        report_details.set("".to_string());
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
                div { class: "panel mb-6 animate-slide-in",
                    div { class: "panel-header", h2 { class: "panel-title", "Publish Content" } }
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
                             label { class: "form-label", "Content (HTML/Markdown)" }
                             textarea { class: "input min-h-[200px] font-mono", value: "{publish_content}", oninput: move |e| publish_content.set(e.value()) }
                         }
                         button { class: "btn btn-primary", onclick: on_publish, "Publish" }
                    }
                }
            }

            // Browser Bar
            div { class: "flex flex-col gap-2 mb-6",
                    div { class: "flex gap-2 mb-6 border-b border-[var(--border-subtle)] pb-2",
                        div {
                            class: if search_mode() == "web" { "nav-button active cursor-pointer" } else { "nav-button cursor-pointer" },
                            onclick: move |_| search_mode.set("web"),
                            "Web Search"
                        }
                        div {
                            class: if search_mode() == "file" { "nav-button active cursor-pointer" } else { "nav-button cursor-pointer" },
                            onclick: move |_| search_mode.set("file"),
                            "File Search"
                        }
                    }
            div { class: "flex gap-2",
                input {
                    class: "input flex-1",
                    placeholder: "Enter URL (sp://) or search terms...",
                    value: "{url_input}",
                    oninput: move |e| url_input.set(e.value()),
                    onkeydown: {
                        let mut submit = submit_action.clone();
                        move |e| {
                            if e.key() == Key::Enter {
                                submit();
                            }
                        }
                    }
                }
                button {
                    class: "btn btn-primary",
                    onclick: {
                        let mut submit = submit_action.clone();
                        move |_| submit()
                    },
                    if is_loading() { "Loading..." } else { "Go" }
                }
                }
            }
            
            // Main Content Area
            div { class: "flex-1 bg-[var(--bg-secondary)] rounded-lg border border-[var(--border-default)] p-4 relative",
                
                // Toolbar
                div { class: "absolute top-4 right-4 flex gap-2",
                    if url_input().starts_with("sp://") {
                        button { 
                            class: "btn btn-sm btn-destructive opacity-80 hover:opacity-100", 
                            onclick: move |_| show_report_modal.set(true),
                            "⚠️ Report" 
                        }
                    }
                }

                // Rendered Content
                if search_mode() == "file" && !app_state.file_search_results.read().is_empty() {
                    div { class: "space-y-4",
                        h3 { class: "text-lg font-bold mb-2", "File Search Results" }
                        for node in app_state.file_search_results.read().iter() {
                             if let crate::backend::dag::DagPayload::File(file) = &node.payload {
                                div { class: "card flex justify-between items-center p-4",
                                    div {
                                        h4 { class: "font-bold text-primary", "{file.name}" }
                                        p { class: "text-sm text-[var(--text-secondary)]", "Type: {file.mime_type} • Size: {file.size} bytes" }
                                    }
                                    a {
                                        class: "btn btn-sm btn-secondary opacity-50 cursor-not-allowed",
                                        // href: "data:{file.mime_type};base64,{file.data}", // FilePayload separates data into blob
                                        "Download (CID: {file.blob_cid})" // Show CID for now
                                    }
                                }
                            }
                        }
                    }
                } else if search_mode() == "web" && !app_state.web_search_results.read().is_empty() {
                    div { class: "space-y-4",
                        h3 { class: "text-lg font-bold mb-2", "Web Search Results" }
                        for node in app_state.web_search_results.read().iter() {
                            if let crate::backend::dag::DagPayload::Web(web) = &node.payload {
                                div { class: "card cursor-pointer hover:bg-[var(--bg-elevated)] transition-colors",
                                    onclick: {
                                        let u = web.url.clone();
                                        let mut submit = submit_action.clone();
                                        move |_| {
                                            url_input.set(u.clone());
                                            submit();
                                        }
                                    },
                                    h4 { class: "font-bold text-primary", "{web.title}" }
                                    p { class: "text-sm text-[var(--text-secondary)]", "{web.description}" }
                                    p { class: "text-xs font-mono mt-1 text-[var(--text-muted)]", "{web.url}" }
                                }
                            }
                        }
                    }
                } else {
                     div { 
                        class: "prose prose-invert max-w-none",
                        dangerous_inner_html: "{_current_content}" 
                    }
                }
            }

            // Report Modal
            if show_report_modal() {
                div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                    div { class: "bg-[var(--bg-default)] p-6 rounded-lg max-w-md w-full border border-[var(--border-default)] shadow-xl",
                        h3 { class: "text-xl font-bold mb-4", "Report Content" }
                        p { class: "mb-4 text-[var(--text-secondary)]", "Reporting: {url_input}" }
                        
                        div { class: "form-group mb-4",
                            label { class: "form-label", "Reason" }
                            select {
                                class: "input",
                                onchange: move |e| report_reason.set(e.value()),
                                option { value: "Spam", "Spam" }
                                option { value: "Illegal Content", "Illegal Content" }
                                option { value: "Harassment", "Harassment" }
                                option { value: "Misinformation", "Misinformation" }
                                option { value: "Other", "Other" }
                            }
                        }
                        
                        div { class: "form-group mb-6",
                            label { class: "form-label", "Details (Optional)" }
                            textarea {
                                class: "input min-h-[100px]",
                                placeholder: "Provide additional context...",
                                value: "{report_details}",
                                oninput: move |e| report_details.set(e.value())
                            }
                        }
                        
                        div { class: "flex justify-end gap-3",
                            button { 
                                class: "btn btn-secondary", 
                                onclick: move |_| show_report_modal.set(false),
                                "Cancel" 
                            }
                            button { 
                                class: "btn btn-destructive", 
                                onclick: on_report_submit,
                                "Submit Report" 
                            }
                        }
                    }
                }
            }

        }
    }
}
