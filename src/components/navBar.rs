use dioxus::prelude::*;
use crate::Route;

#[component]
pub fn NavComponent() -> Element {
    rsx! {
        nav {id: "main-nav",
            
            Link {
                to: Route::HomeComponent {},
                id: "nav-home",
                "Home"
            }
            
            Link {
                to: Route::MessagingComponent {},
                id: "nav-messaging",
                "Messaging"
            }
        }
        Outlet::<Route> {}
    }
}