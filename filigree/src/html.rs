//! Utilities for genrating HTML

use std::fmt::{Display, Write};

use maud::{Escaper, Render};

/// Render a list of strings as HTML
#[derive(Default)]
pub struct HtmlList<'item, 'class, T: Display> {
    /// The items to render
    pub items: &'item [T],
    /// A class to place on the `ul` element
    pub ul_class: &'class str,
    /// A class to place on the `li` elements
    pub li_class: &'class str,
    /// Whether to render the `ul` element
    pub render_ul: bool,
}

impl<'item, 'class, T: Display> HtmlList<'item, 'class, T> {
    /// Create a new [HtmlList] with no classes or enclosing `ul` element.
    pub fn new(items: &'item [T]) -> Self {
        Self {
            items,
            ul_class: "",
            li_class: "",
            render_ul: false,
        }
    }

    /// Set the classes that will render on each `li` element.
    pub fn li_class(mut self, class: &'class str) -> Self {
        self.li_class = class;
        self
    }

    /// Enable `ul` rendering and set the classes that will render on the enclosing `ul` element.
    pub fn ul_class(mut self, class: &'class str) -> Self {
        self.render_ul = true;
        self.ul_class = class;
        self
    }

    /// Enable `ul` rendering
    pub fn render_ul(mut self) -> Self {
        self.render_ul = true;
        self
    }
}

impl Render for HtmlList<'_, '_, String> {
    fn render_to(&self, buf: &mut String) {
        if self.items.is_empty() {
            return;
        }

        opening_element_with_class(buf, "ul", self.ul_class);
        for item in self.items {
            opening_element_with_class(buf, "li", self.li_class);
            Escaper::new(buf).write_str(item).unwrap();
            buf.push_str("</li>");
        }

        if self.render_ul {
            buf.push_str("</ul>");
        }
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
            Escaper::new(buf).write_str(self.class).unwrap();
            buf.push('"');
        }

        let icon_rest = &self.svg[4..];
        buf.push_str(icon_rest);
    }
}

fn opening_element_with_class(buf: &mut String, el: &str, class: &str) {
    buf.push('<');
    buf.push_str(el);
    if !class.is_empty() {
        buf.push_str(" class=\"");
        Escaper::new(buf).write_str(class).unwrap();
        buf.push('"');
    }
    buf.push('>');
}
