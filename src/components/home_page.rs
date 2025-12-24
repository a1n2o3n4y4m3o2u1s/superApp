use dioxus::prelude::*;
use crate::backend::AppCmd;
use crate::backend::dag::{DagPayload, PostPayload, DagNode};
use base64::{Engine as _, engine::general_purpose};

#[component]
pub fn HomeComponent() -> Element {
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    let app_state = use_context::<crate::components::AppState>();
    
    use crate::components::common::{BlobImage, StoryCircle, StoryViewer};
    
    let mut active_feed_tab = use_signal(|| "global".to_string());
    
    let posts = if active_feed_tab() == "following" {
        app_state.following_posts
    } else {
        app_state.posts
    };
    
    let mut new_post_content = use_signal(|| "".to_string());
    let mut attached_cids = use_signal(|| Vec::<String>::new());
    let mut announcement = use_signal(|| false);
    
    let mut last_processed_blob = use_signal(|| None::<String>);
    let is_uploading_story = use_signal(|| false);
    
    // Stories state
    let users_stories = app_state.stories;
    let mut viewed_story = use_signal(|| None::<DagNode>);
    let mut seen_stories = app_state.seen_stories;
    let user_profiles = app_state.user_profiles;
    
     use_effect(move || {
        if let Some(blob_id) = app_state.last_created_blob.read().clone() {
            if last_processed_blob() != Some(blob_id.clone()) {
                 if !is_uploading_story() {
                     attached_cids.write().push(blob_id.clone());
                 }
                 last_processed_blob.set(Some(blob_id));
            }
        }
    });

    let cmd_tx_clone = cmd_tx.clone();
    use_effect(move || {
        let _ = cmd_tx_clone.send(AppCmd::FetchPosts);
        let _ = cmd_tx_clone.send(AppCmd::FetchStories);
        let _ = cmd_tx_clone.send(AppCmd::FetchFollowingPosts);
    });

    let cmd_tx_feed = cmd_tx.clone();
    use_effect(move || {
        if active_feed_tab() == "following" {
            let _ = cmd_tx_feed.send(AppCmd::FetchFollowingPosts);
        } else {
            let _ = cmd_tx_feed.send(AppCmd::FetchPosts);
        }
    });

    let on_submit = {
        let cmd_tx = cmd_tx.clone();
        move |_| {
            if !new_post_content().is_empty() || !attached_cids().is_empty() {
                let cmd = AppCmd::PublishPost {
                    content: new_post_content(),
                    attachments: attached_cids().clone(),
                    geohash: None,
                    announcement: announcement(),
                };
                if let Err(e) = cmd_tx.send(cmd) {
                    eprintln!("Failed to send PublishPost command: {:?}", e);
                } else {
                    new_post_content.set("".to_string());
                    attached_cids.set(Vec::new());
                    announcement.set(false);
                }
            }
        }
    };

    let upload_file = {
        let cmd_tx = cmd_tx.clone();
        move |evt: Event<FormData>| {
            let cmd_tx = cmd_tx.clone();
            let files: Vec<_> = evt.files().into_iter().collect();
            spawn(async move {
                for file_data in files {
                    let file_name = file_data.name();
                    let mime_type = file_data.content_type().unwrap_or_else(|| "image/jpeg".to_string());
                    
                    if let Ok(file_bytes) = file_data.read_bytes().await {
                        let data = general_purpose::STANDARD.encode(&file_bytes);
                        // Just publish blob normally for posts
                        if let Err(e) = cmd_tx.send(AppCmd::PublishBlob { mime_type, data }) {
                            eprintln!("Failed to send PublishBlob command: {:?}", e);
                        }
                    }
                }
            });
        }
    };
    
    let upload_story = {
        let cmd_tx = cmd_tx.clone();
        move |evt: Event<FormData>| {
            let cmd_tx = cmd_tx.clone();
            let files: Vec<_> = evt.files().into_iter().collect();
            spawn(async move {
                for file_data in files {
                    let mime_type = file_data.content_type().unwrap_or_else(|| "image/jpeg".to_string());
                    if let Ok(file_bytes) = file_data.read_bytes().await {
                        let data = general_purpose::STANDARD.encode(&file_bytes);
                        
                         // 1. Publish Blob
                         if let Err(e) = cmd_tx.send(AppCmd::PublishBlob { mime_type, data }) {
                             eprintln!("Failed to send PublishBlob command: {:?}", e);
                         }
                    }
                }
            });
        }
    };
    
    let mut is_uploading_story = use_signal(|| false);

    let cmd_tx_story = cmd_tx.clone();
    use_effect(move || {
        if let Some(blob_id) = app_state.last_created_blob.read().clone() {
            if last_processed_blob() != Some(blob_id.clone()) {
                 if is_uploading_story() {
                     // Publish Story
                     let cmd = AppCmd::PublishStory {
                         media_cid: blob_id.clone(),
                         caption: "".to_string(),
                         geohash: if *app_state.geohash.read() != "Global" {
                             Some(app_state.geohash.read().clone())
                         } else {
                             None
                         },
                     };
                     let _ = cmd_tx_story.send(cmd);
                     let _ = cmd_tx_story.send(AppCmd::FetchStories);
                     is_uploading_story.set(false);
                 } else {
                     // Append to post
                     attached_cids.write().push(blob_id.clone());
                 }
                 last_processed_blob.set(Some(blob_id));
            }
        }
    });

    let following = app_state.following;
    let local_peer_id_val = app_state.local_peer_id.read().clone();

    rsx! {
        div { class: "page-container py-8 animate-fade-in",
            
            // Stories Bar
            div { class: "mb-8 overflow-x-auto whitespace-nowrap pb-4 scrollbar-hide",
                div { class: "flex gap-4",
                    // Add Story Button
                    div { class: "inline-block relative",
                         div { class: "w-16 h-16 rounded-full bg-[var(--bg-secondary)] border-2 border-[var(--primary)] flex items-center justify-center cursor-pointer hover:bg-[var(--bg-hover)] transition-colors",
                            if is_uploading_story() {
                                div { class: "w-6 h-6 border-2 border-[var(--primary)] border-t-transparent rounded-full animate-spin" }
                            } else {
                                span { class: "text-2xl font-bold text-[var(--primary)]", "+" }
                            }
                            input {
                                class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                                r#type: "file",
                                accept: "image/*,video/*",
                                disabled: is_uploading_story(),
                                onchange: move |e| {
                                    is_uploading_story.set(true);
                                    upload_story(e);
                                }
                            }
                         }
                         div { class: "text-xs text-center mt-1 truncate w-16", if is_uploading_story() { "..." } else { "You" } }
                    }
                    
                    // Stories List
                    for node in users_stories() {
                        {
                            let is_seen = seen_stories.read().contains(&node.id);
                            let local_peer_id_val = local_peer_id_val.clone();
                            let following = following.read().clone();
                            let mut seen_stories = seen_stories.clone();
                            let mut viewed_story = viewed_story.clone();

                            if node.author == local_peer_id_val || following.contains(&node.author) {
                                rsx! {
                                    StoryCircle {
                                        node: node.clone(),
                                        is_seen: is_seen,
                                        onclick: move |n: DagNode| {
                                            seen_stories.write().insert(n.id.clone());
                                            viewed_story.set(Some(n));
                                        }
                                    }
                                }
                            } else {
                                rsx! { div {} }
                            }
                        }
                    }
                }
            }
            
            // Story Viewer Modal
            if let Some(node) = viewed_story() {
                StoryViewer {
                    node: node,
                    on_close: move |_| viewed_story.set(None)
                }
            }

            // Compose area
            div { class: "panel mb-8",
                div { class: "panel-header",
                    h2 { class: "panel-title", "Share something" }
                }
                
                div { class: "form-group",
                    textarea {
                        class: "input",
                        style: "min-height: 100px; resize: none;",
                        placeholder: "What's on your mind?",
                        value: "{new_post_content}",
                        oninput: move |e| new_post_content.set(e.value())
                    }
                }
                
                div { class: "form-group flex items-center gap-2 mb-4 p-2 rounded bg-[var(--bg-secondary)] border border-[var(--border-color)]",
                    input {
                        r#type: "checkbox",
                        checked: "{announcement}",
                        onchange: move |e| announcement.set(e.checked()),
                        class: "w-4 h-4 cursor-pointer"
                    }
                    label { class: "text-sm font-bold cursor-pointer", onclick: move |_| announcement.set(!announcement()), "📢 Official Announcement (Elected Officials Only)" }
                }
                
                if !attached_cids().is_empty() {
                    div { class: "flex gap-3 mb-4 overflow-x-auto pb-2",
                        for cid in attached_cids() {
                            div { class: "relative w-20 h-20 rounded-lg overflow-hidden border border-[var(--border-default)] flex-shrink-0",
                                BlobImage { cid: cid.clone() }
                                button {
                                    class: "absolute top-1 right-1 w-5 h-5 bg-black/70 hover:bg-black rounded-full flex items-center justify-center text-xs",
                                    onclick: move |_| {
                                         attached_cids.write().retain(|c| c != &cid);
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }

                div { class: "flex justify-between items-center pt-4 border-t border-[var(--border-subtle)]",
                    div { class: "relative",
                        button { class: "btn btn-secondary btn-sm",
                            "📷 Add Image"
                        }
                        input {
                            class: "absolute inset-0 w-full h-full opacity-0 cursor-pointer",
                            r#type: "file",
                            accept: "image/*",
                            onchange: upload_file
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: on_submit,
                        "Post"
                    }
                }
            }

            // Feed
            div { class: "panel",
                div { class: "panel-header flex gap-4",
                    button { 
                        class: if active_feed_tab() == "global" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-ghost" },
                        onclick: move |_| active_feed_tab.set("global".to_string()),
                        "Global"
                    }
                    button { 
                        class: if active_feed_tab() == "following" { "btn btn-sm btn-primary" } else { "btn btn-sm btn-ghost" },
                        onclick: move |_| active_feed_tab.set("following".to_string()),
                        "Following"
                    }
                }
                
                if posts().is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", "📝" }
                        p { class: "empty-state-title", "No posts yet" }
                        p { class: "empty-state-text", "Be the first to share something!" }
                    }
                } else {
                    for node in posts() {
                        if let DagPayload::Post(PostPayload { content, attachments, announcement, .. }) = &node.payload {
                            {
                                let post_id = node.id.clone();
                                let app_state = app_state.clone();
                                let cmd_tx = cmd_tx.clone();
                                let is_announcement = *announcement;
                                
                                let mut show_reply = use_signal(|| false);
                                let mut reply_content = use_signal(|| "".to_string());
                                
                                // Get comments
                                let comments_map = app_state.comments.read();
                                let comments = comments_map.get(&post_id).cloned().unwrap_or_default();
                                let comment_count = comments.len();
                                drop(comments_map);
                                
                                // Get likes
                                let likes_map = app_state.likes.read();
                                let (like_count, is_liked_by_me) = likes_map.get(&post_id).cloned().unwrap_or((0, false));
                                drop(likes_map);
                                
                                // Fetch on mount
                                use_effect({
                                    let cmd_tx = cmd_tx.clone();
                                    let pid = post_id.clone();
                                    move || {
                                        let _ = cmd_tx.send(AppCmd::FetchComments { parent_id: pid.clone() });
                                        let _ = cmd_tx.send(AppCmd::FetchLikes { target_id: pid.clone() });
                                    }
                                });

                                rsx! {
                                    div { 
                                        class: if is_announcement { 
                                            "post border-2 border-yellow-500 bg-[var(--bg-secondary)] shadow-lg shadow-yellow-500/10" 
                                        } else { 
                                            "post" 
                                        },
                                        key: "{node.id}",
                                        
                                        if is_announcement {
                                            div { class: "bg-yellow-500 text-black text-xs font-bold px-3 py-1 -mt-4 -ml-4 -mr-4 mb-4 rounded-t-lg flex items-center gap-2",
                                                span { "📢 OFFICIAL ANNOUNCEMENT" }
                                            }
                                        }
                                        
                                        div { class: "post-header",
                                            div { class: "avatar",
                                                "{node.author.get(0..2).unwrap_or(\"??\")}"
                                            }
                                            div { class: "flex-1",
                                                button {
                                                    class: "post-author cursor-pointer hover:underline bg-transparent border-none p-0 text-left",
                                                    onclick: {
                                                        let peer_id = node.author.clone();
                                                        let mut app_state = app_state.clone();
                                                        move |_| {
                                                            app_state.browser_url.set(format!("sp://profile.super/{}", peer_id));
                                                        }
                                                    },
                                                    "{node.author.get(0..12).unwrap_or(&node.author)}..."
                                                }
                                            }
                                            span { class: "post-time", "{node.timestamp}" }
                                        }
                                        
                                        p { class: "post-content", "{content}" }
                                        
                                        if !attachments.is_empty() {
                                            div { class: "post-attachments",
                                                for cid in attachments {
                                                    div { class: "post-attachment",
                                                        BlobImage { cid: cid.clone() }
                                                    }
                                                }
                                            }
                                        }
                                        
                                        // Like and Comment buttons
                                        div { class: "post-actions",
                                            button { 
                                                class: if is_liked_by_me { "post-action-btn post-action-btn--liked" } else { "post-action-btn" },
                                                onclick: {
                                                    let cmd_tx = cmd_tx.clone();
                                                    let pid = node.id.clone();
                                                    move |_| {
                                                        let _ = cmd_tx.send(AppCmd::LikePost { target_id: pid.clone(), remove: is_liked_by_me });
                                                    }
                                                },
                                                span { class: "icon", if is_liked_by_me { "❤️" } else { "🤍" } }
                                                span { class: "count", "{like_count}" }
                                            }
                                            button { 
                                                class: "post-action-btn",
                                                onclick: move |_| show_reply.set(!show_reply()),
                                                span { class: "icon", "💬" }
                                                span { class: "count", "{comment_count}" }
                                            }
                                        }
                                        
                                        // Comments section
                                        if !comments.is_empty() || show_reply() {
                                            div { class: "post-comments",
                                                // Existing comments
                                                for c_node in comments {
                                                    CommentComponent { key: "{c_node.id}", node: c_node }
                                                }
                                                
                                                // Reply input
                                                if show_reply() {
                                                    div { class: "post-comment-input",
                                                        input {
                                                            class: "input",
                                                            placeholder: "Write a comment...",
                                                            value: "{reply_content}",
                                                            oninput: move |e| reply_content.set(e.value()),
                                                            onkeypress: {
                                                                let cmd_tx = cmd_tx.clone();
                                                                let pid = node.id.clone();
                                                                move |e: KeyboardEvent| {
                                                                    if e.key() == Key::Enter && !reply_content().is_empty() {
                                                                        let _ = cmd_tx.send(AppCmd::PostComment { parent_id: pid.clone(), content: reply_content() });
                                                                        reply_content.set("".to_string());
                                                                        let _ = cmd_tx.send(AppCmd::FetchComments { parent_id: pid.clone() });
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        button {
                                                            class: "btn btn-primary btn-sm",
                                                            onclick: {
                                                                let cmd_tx = cmd_tx.clone();
                                                                let pid = node.id.clone();
                                                                move |_| {
                                                                    if !reply_content().is_empty() {
                                                                        let _ = cmd_tx.send(AppCmd::PostComment { parent_id: pid.clone(), content: reply_content() });
                                                                        reply_content.set("".to_string());
                                                                        let _ = cmd_tx.send(AppCmd::FetchComments { parent_id: pid.clone() });
                                                                    }
                                                                }
                                                            },
                                                            "Send"
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


#[component]
fn CommentComponent(node: DagNode) -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let cmd_tx = use_context::<tokio::sync::mpsc::UnboundedSender<AppCmd>>();
    
    let mut show_reply = use_signal(|| false);
    let mut reply_content = use_signal(|| "".to_string());
    
    // Get replies
    let comments_map = app_state.comments.read();
    let replies = comments_map.get(&node.id).cloned().unwrap_or_default();
    drop(comments_map);

    let comment_content = if let DagPayload::Comment(c) = &node.payload {
        c.content.clone()
    } else {
        "".to_string()
    };

    let timestamp = node.timestamp.format("%H:%M").to_string();

    // Fetch replies on mount
    let cmd_tx_effect = cmd_tx.clone();
    let node_id = node.id.clone();
    use_effect(move || {
       let _ = cmd_tx_effect.send(AppCmd::FetchComments { parent_id: node_id.clone() }); 
    });

    rsx! {
        div { class: "comment",
            div { class: "comment-avatar",
                "{node.author.get(0..2).unwrap_or(\"??\")}"
            }
            div { class: "comment-body",
                div { class: "comment-header",
                    span { class: "comment-author", "{node.author.get(0..8).unwrap_or(\"??\")}..." }
                    span { class: "comment-time", "{timestamp}" }
                }
                p { class: "comment-content", "{comment_content}" }
                
                div { class: "comment-actions",
                    button { 
                        class: "comment-reply-btn",
                        onclick: move |_| show_reply.set(!show_reply()),
                        if show_reply() { "Cancel" } else { "Reply" }
                    }
                    if !replies.is_empty() {
                        span { class: "comment-reply-btn", " · {replies.len()} replies" }
                    }
                }
                
                // Reply input
                if show_reply() {
                    div { class: "post-comment-input",
                        input {
                            class: "input",
                            placeholder: "Reply...",
                            value: "{reply_content}",
                            oninput: move |e| reply_content.set(e.value()),
                            onkeypress: {
                                let cmd_tx = cmd_tx.clone();
                                let pid = node.id.clone();
                                move |e: KeyboardEvent| {
                                    if e.key() == Key::Enter && !reply_content().is_empty() {
                                        let _ = cmd_tx.send(AppCmd::PostComment { parent_id: pid.clone(), content: reply_content() });
                                        reply_content.set("".to_string());
                                        let _ = cmd_tx.send(AppCmd::FetchComments { parent_id: pid.clone() });
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Nested replies
                if !replies.is_empty() {
                    div { class: "comment-replies",
                        for reply_node in replies {
                            CommentComponent { key: "{reply_node.id}", node: reply_node }
                        }
                    }
                }
            }
        }
    }
}