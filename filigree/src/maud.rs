//! Utilities for working with Maud

use maud::{Markup, Render};

/// A lazy renderer for Maud templates. Use with the [maud_lazy] macro.
pub struct Lazy<F>
where
    F: Fn() -> Markup,
{
    /// The function that will render the markup
    pub contents: F,
}

/// Render a maud template lazily
///
/// ```
/// # use maud::{PreEscaped, Render};
/// # use filigree::maud_lazy;
/// let value = "hello";
/// let lazy = maud_lazy! {
///     p { (value) " world" }
/// };
/// assert_eq!(lazy.render().0, "<p>hello world</p>");
/// ```
///
/// You can also create a move closure if you need to.
///
/// ```
/// # use maud::{PreEscaped, Render};
/// # use filigree::maud_lazy;
/// let value = "hello";
/// let lazy = maud_lazy! { move
///     p { (value) " world" }
/// };
/// assert_eq!(lazy.render().0, "<p>hello world</p>");
/// ```
#[macro_export]
macro_rules! maud_lazy {
    (move $($ex: tt)*) => {
        $crate::maud::Lazy {
            contents: move || maud::html! { $($ex)* },
        }
    };

    ($($ex: tt)*) => {
        $crate::maud::Lazy {
            contents: || maud::html! { $($ex)* },
        }
    };

}

impl<F> Render for Lazy<F>
where
    F: Fn() -> Markup,
{
    fn render_to(&self, buf: &mut String) {
        (self.contents)().render_to(buf);
    }
}

/// Render an SVG with custom classes. This assumes that the icon SVG starts with
/// the string "<svg".
pub struct Svg<'svg, 'class> {
    svg: &'svg str,
    class: &'class str,
}

impl<'svg, 'class> Svg<'svg, 'class> {
    /// Render with the `fill-current` class
    pub fn new(svg: &'svg str) -> Self {
        Svg {
            svg,
            class: "fill-current",
        }
    }

    /// An icon that renders with a custom class
    pub fn class(svg: &'svg str, class: &'class str) -> Self {
        Svg { svg, class }
    }
}

impl<'svg, 'class> Render for Svg<'svg, 'class> {
    fn render_to(&self, buf: &mut String) {
        buf.push_str("<svg");

        if !self.class.is_empty() {
            buf.push_str(" class=\"");
            buf.push_str(self.class);
            buf.push('"');
        }

        let icon_rest = &self.svg[4..];
        buf.push_str(icon_rest);
    }
}
