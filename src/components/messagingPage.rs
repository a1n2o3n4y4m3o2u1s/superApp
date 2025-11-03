use dioxus::prelude::*;

#[component]
pub fn MessagingComponent() -> Element {
    rsx! { 
        div {id: "messaging-page",
            h1 {"Messaging page"}
        }
    }
}