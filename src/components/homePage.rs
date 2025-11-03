use dioxus::prelude::*;

#[component]
pub fn HomeComponent() -> Element {
    rsx! { 
        div {id: "home-page",
            h1 {"Welcome to Superapp!"}
        }
    }
}