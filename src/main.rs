mod components;

use components::homePage;
use components::messagingPage;
use components::navBar;

use homePage::HomeComponent;
use messagingPage::MessagingComponent;
use navBar::NavComponent;

use dioxus::prelude::*;

#[derive(Routable, Clone, PartialEq)]
enum Route {
    #[layout(NavComponent)]
    #[route("/")]
    HomeComponent {},
    #[route("/messaging")]
    MessagingComponent {},
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet {href: asset!("/assets/main.css")}
        Router::<Route> {}
    }
}