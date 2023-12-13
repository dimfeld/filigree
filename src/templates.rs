use std::sync::OnceLock;

use tera::Tera;

static CELL: OnceLock<Tera> = OnceLock::new();

pub fn get_tera() -> &'static Tera {
    CELL.get_or_init(create_tera)
}

fn create_tera() -> Tera {
    let mut tera = Tera::default();

    tera.add_raw_templates(vec![
        ("delete.sql.tera", include_str!("model/delete.sql.tera")),
        ("insert.sql.tera", include_str!("model/insert.sql.tera")),
        (
            "select_one.sql.tera",
            include_str!("model/select_one.sql.tera"),
        ),
        ("update.sql.tera", include_str!("model/update.sql.tera")),
        ("model_macros.tera", include_str!("model/model_macros.tera")),
    ])
    .expect("Could not add templates");

    tera
}
