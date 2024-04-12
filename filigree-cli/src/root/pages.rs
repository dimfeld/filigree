use std::path::PathBuf;

use error_stack::Report;
use itertools::Itertools;
use rayon::prelude::*;

use crate::{
    config::pages::Page,
    templates::Renderer,
    write::{RenderedFile, RenderedFileLocation},
    Error,
};

pub const PAGE_PATH: &str = "root/pages/_page.rs.tera";
pub const NON_PAGE_NODE_PATH: &str = "root/pages/_intermediate_non_page.rs.tera";

#[derive(Debug)]
struct ModuleTree<'a> {
    name: &'a str,
    path: String,
    page: Option<&'a Page>,
    children: Vec<ModuleTree<'a>>,
}

impl<'a> ModuleTree<'a> {
    fn add_path<'b>(&'b mut self, page: &'a Page, path: Vec<&'a str>, index: usize) {
        if index == path.len() {
            self.page = Some(page);
            return;
        }

        let this_name = path[index];

        if let Some(existing) = self.children.iter_mut().rfind(|c| c.name == this_name) {
            existing.add_path(page, path, index + 1);
        } else {
            let node = ModuleTree {
                name: this_name,
                path: format!("/{}", path[0..=index].join("/")),
                page: None,
                children: vec![],
            };

            self.children.push(node);

            self.children
                .last_mut()
                .unwrap()
                .add_path(page, path, index + 1);
        };
    }

    fn result<'b>(&'b mut self, output: &'b mut Vec<ModuleTreeResult<'a>>) {
        let submodules = self
            .children
            .iter()
            .map(|c| c.name.replace(":", "_"))
            .sorted_by(|a, b| a.cmp(b))
            .collect();

        output.push(ModuleTreeResult {
            name: self.name.to_string(),
            path: self.path.clone(),
            page: self.page.take(),
            submodules,
        });

        for child in &mut self.children {
            child.result(output);
        }
    }
}

#[derive(Debug)]
struct ModuleTreeResult<'a> {
    name: String,
    path: String,
    page: Option<&'a Page>,
    submodules: Vec<String>,
}

pub fn render_pages(
    pages: Vec<Page>,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let mut module_tree = ModuleTree {
        name: "home",
        path: "/".to_string(),
        page: None,
        children: vec![],
    };

    for page in &pages {
        let path = page.config.path.segments().collect::<Vec<_>>();
        module_tree.add_path(page, path, 0);
    }

    let mut output = Vec::with_capacity(pages.len());

    module_tree.result(&mut output);
    output.sort_by(|a, b| a.name.cmp(&b.name));

    let root_page = output
        .iter()
        .find(|page| page.path == "/")
        .expect("finding root page");
    let root_page_context = root_page
        .page
        .map(|page| page.template_context(root_page.submodules.clone()))
        .expect("creating template context for root page");

    let root_page_output = renderer.render_with_full_path(
        PathBuf::from("src/pages/mod.rs"),
        "root/pages/mod.rs.tera",
        RenderedFileLocation::Rust,
        &tera::Context::from_value(root_page_context).unwrap(),
    )?;

    let mut output = output
        .into_par_iter()
        .filter(|module| module.path != "/")
        .map(|module| {
            let (context, template) = if let Some(page) = module.page {
                let context = page.template_context(module.submodules);
                let context = tera::Context::from_value(context).unwrap();
                (context, PAGE_PATH)
            } else {
                let mut context = tera::Context::new();
                context.insert("has_handler", &false);
                context.insert("submodules", &module.submodules);
                // todo actually need a modules-only template
                (context, NON_PAGE_NODE_PATH)
            };

            let path = module.path.replace(':', "_");
            let output_path = format!("src/pages{}.rs", path);
            renderer.render_with_full_path(
                PathBuf::from(output_path),
                template,
                RenderedFileLocation::Rust,
                &context,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    output.push(root_page_output);

    Ok(output)
}
