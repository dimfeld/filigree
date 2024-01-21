// Code in this file inspired by [axum-jsonschema](https://github.com/tamasfe/aide/blob/master/crates/axum-jsonschema/src/lib.rs)

use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, VecDeque},
};

use jsonschema::{
    output::{BasicOutput, ErrorDescription, OutputUnit},
    JSONSchema,
};
use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    JsonSchema,
};

thread_local! {
    static SCHEMAS: RefCell<Schemas> = RefCell::new(Schemas::new());
}

struct Schemas {
    generator: SchemaGenerator,
    schemas: HashMap<TypeId, JSONSchema>,
}

impl Schemas {
    fn new() -> Self {
        Self {
            generator: SchemaSettings::draft07()
                .with(|g| g.inline_subschemas = true)
                .into_generator(),
            schemas: HashMap::new(),
        }
    }
}

pub type SchemaErrors = VecDeque<OutputUnit<ErrorDescription>>;

pub fn validate<T: JsonSchema>(input: &serde_json::Value) -> Result<(), SchemaErrors> {
    SCHEMAS.with(|sch| {
        let sch = &mut *sch.borrow_mut();
        let schema = sch.schemas.entry(TypeId::of::<T>()).or_insert_with(|| {
            match JSONSchema::compile(
                &serde_json::to_value(sch.generator.root_schema_for::<T>()).unwrap(),
            ) {
                Ok(schema) => schema,
                Err(err) => JSONSchema::compile(&serde_json::json!({})).unwrap(),
            }
        });

        match schema.apply(input).basic() {
            BasicOutput::Valid(_) => Ok(()),
            BasicOutput::Invalid(err) => Err(err),
        }
    })
}
