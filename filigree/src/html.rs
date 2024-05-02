//! Utilities for genrating HTML

use std::fmt::Display;

use minijinja::HtmlEscape;

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

impl Display for HtmlList<'_, '_, String> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.items.is_empty() {
            return Ok(());
        }

        opening_element_with_class(f, "ul", self.ul_class)?;
        for item in self.items {
            opening_element_with_class(f, "li", self.li_class)?;
            write!(f, "{}</li>", minijinja::HtmlEscape(item))?;
        }

        if self.render_ul {
            write!(f, "</ul>")?;
        }
        Ok(())
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

impl<'svg, 'class> Display for Svg<'svg, 'class> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<svg")?;

        if !self.class.is_empty() {
            write!(f, r##" class="{}""##, HtmlEscape(self.class))?;
        }

        let icon_rest = &self.svg[4..];
        write!(f, "{icon_rest}")?;
        Ok(())
    }
}

fn opening_element_with_class(
    fmt: &mut std::fmt::Formatter<'_>,
    el: &str,
    class: &str,
) -> std::fmt::Result {
    write!(fmt, "<{el}")?;
    if !class.is_empty() {
        write!(fmt, r##" class="{}""##, HtmlEscape(class))?;
    }
    write!(fmt, ">")?;
    Ok(())
}
