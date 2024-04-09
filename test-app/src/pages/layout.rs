use maud::{html, Markup, DOCTYPE};

use crate::auth::Authed;

/// The HTML shell that every page should be wrapped in to enable basic functionality.
pub fn page_wrapper(title: &str, slot: Markup) -> Markup {
    html! {
         (DOCTYPE)
         html {
             head {
                 meta charset="utf-8";
                 meta name="viewport" content="width=device-width, initial-scale=1";
                 title { (title) }
             }
             body {
                 (slot)
             }
         }
    }
}

/// The root layout of the application
pub fn root_layout(auth: Option<&Authed>, slot: Markup) -> Markup {
    html! {
        (slot)
    }
}

/// The root layout of the application, as a full HTML page
pub fn root_layout_page(auth: Option<&Authed>, title: &str, slot: Markup) -> Markup {
    page_wrapper(title, root_layout(auth, slot))
}
