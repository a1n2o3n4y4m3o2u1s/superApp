use dioxus::prelude::*;
use crate::Route;

#[component]
pub fn NavComponent() -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let verification_status = app_state.verification_status.read();

    rsx! {
        div { class: "min-h-screen flex flex-col",
            nav { class: "nav-bar",
                div { class: "page-container",
                    // Logo section
                    div { class: "nav-logo cursor-pointer",
                        onclick: move |_| {
                            let mut app_state = use_context::<crate::components::AppState>();
                            app_state.browser_url.set("sp://welcome".to_string());
                        },
                        div { class: "logo-icon" }
                        span { class: "logo-text", "SuperApp" }
                        match *verification_status {
                            crate::backend::VerificationStatus::Founder => rsx! {
                                span { class: "badge badge-founder ml-2", "Founder" }
                            },
                            crate::backend::VerificationStatus::Verified => rsx! {
                                span { class: "badge badge-verified ml-2", "Verified" }
                            },
                            _ => rsx! {}
                        }
                    }
                }
            }
            
            div { class: "fixed-header-spacer" }
            
            div { class: "flex-1",
                Outlet::<Route> {}
            }
        }
    }
}