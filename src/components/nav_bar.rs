use dioxus::prelude::*;
use crate::Route;

#[component]
pub fn NavComponent() -> Element {
    let app_state = use_context::<crate::components::AppState>();
    let verification_status = app_state.verification_status.read();
    let is_unverified = matches!(*verification_status, crate::backend::VerificationStatus::Unverified);

    rsx! {
        div { class: "min-h-screen flex flex-col",
            nav { class: "nav-bar",
                div { class: "page-container",
                    // Logo section
                    div { class: "nav-logo",
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

                    // Navigation links
                    div { class: "nav-links",
                        if !is_unverified {
                            Link {
                                to: Route::HomeComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Home"
                            }
                            Link {
                                to: Route::GeohashComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Local"
                            }
                            Link {
                                to: Route::BrowserComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Web"
                            }
                            Link {
                                to: Route::MarketplaceComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Market"
                            }
                            Link {
                                to: Route::MessagingComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Messages"
                            }
                            Link {
                                to: Route::ProfileComponent {},
                                class: "nav-link",
                                active_class: "active",
                                "Profile"
                            }
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