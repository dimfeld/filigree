use std::path::PathBuf;

use error_stack::Report;
use itertools::Itertools;
use rayon::prelude::*;
use serde_json::json;

use crate::{
    config::pages::PageConfig,
    templates::Renderer,
    write::{RenderedFile, RenderedFileLocation},
    Error,
};

pub const PAGE_PATH: &str = "root/pages/_page.rs.tera";
pub const NON_PAGE_NODE_PATH: &str = "root/pages/intermediate_non_page.rs.tera";

struct ModuleTree<'a> {
    name: &'a str,
    path: String,
    page: Option<&'a PageConfig>,
    children: Vec<ModuleTree<'a>>,
}

impl<'a> ModuleTree<'a> {
    fn add_path<'b>(&'b mut self, page: &'a PageConfig, path: Vec<&'a str>, index: usize) {
        if index == path.len() - 1 {
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
        let submodules = self.children.iter().map(|c| c.name).collect();

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

struct ModuleTreeResult<'a> {
    name: String,
    path: String,
    page: Option<&'a PageConfig>,
    submodules: Vec<&'a str>,
}

pub fn render_pages(
    pages: Vec<PageConfig>,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let mut module_tree = ModuleTree {
        name: "",
        path: String::new(),
        page: None,
        children: vec![],
    };

    for page in &pages {
        let path = page.path.segments().collect::<Vec<_>>();
        module_tree.add_path(page, path, 0);
    }

    let mut output = Vec::with_capacity(pages.len());

    let root_modules = module_tree
        .children
        .iter()
        .map(|c| c.name.to_string())
        .sorted_by(|a, b| a.cmp(&b))
        .collect::<Vec<_>>();

    for child in &mut module_tree.children {
        child.result(&mut output);
    }

    output.sort_by(|a, b| a.name.cmp(&b.name));

    let root_context = tera::Context::from_value(json!({
        "root_modules": &root_modules
    }))
    .unwrap();

    let mut output = output
        .into_par_iter()
        .map(|module| {
            let (context, template) = if let Some(page) = module.page {
                let context = page.template_context(module.submodules);
                let context = tera::Context::from_value(context).unwrap();
                (context, PAGE_PATH)
            } else {
                let mut context = tera::Context::new();
                context.insert("submodules", &module.submodules);
                // todo actually need a modules-only template
                (context, NON_PAGE_NODE_PATH)
            };

            let path = module.path.replace(':', "_");
            let output_path = format!("src/pages/{}.rs", path);
            renderer.render_with_full_path(
                PathBuf::from(output_path),
                template,
                RenderedFileLocation::Rust,
                &context,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    let root_page = renderer.render_with_full_path(
        PathBuf::from("src/pages/mod.rs"),
        "root/pages/mod.rs.tera",
        RenderedFileLocation::Rust,
        &root_context,
    )?;
    output.push(root_page);

    Ok(output)
}
