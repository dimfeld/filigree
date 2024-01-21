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
    schema::RootSchema,
    JsonSchema,
};

thread_local! {
    static SCHEMAS: RefCell<Schemas> = RefCell::new(Schemas::new());
}

struct SchemaInfo {
    schema: JSONSchema,
    array_fields: Vec<String>,
}

struct Schemas {
    generator: SchemaGenerator,
    schemas: HashMap<TypeId, SchemaInfo>,
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

/// Validate a JSON value against a JSON schema
/// If `coerce_arrays` is true, non-array values will be coerced to `Vec<_>` if the schema
/// specifies an array. This should be used when the original input format is not completely
/// self-describing, such as `application/x-www-form-urlencoded`.
pub fn validate<T: JsonSchema + 'static>(
    input: &mut serde_json::Value,
    coerce_arrays: bool,
) -> Result<(), SchemaErrors> {
    SCHEMAS.with(|sch| {
        let sch = &mut *sch.borrow_mut();
        let schema = sch
            .schemas
            .entry(TypeId::of::<T>())
            .or_insert_with(|| compile_schema(sch.generator.root_schema_for::<T>()));

        if coerce_arrays {
            // The format this was serialized from can't distinguish between singletons and single element
            // arrays, so coerce them according to the schema.
            for field in &schema.array_fields {
                let Some(data) = input.get_mut(field) else {
                    continue;
                };

                match data {
                    serde_json::Value::Array(_) => {}
                    // Assuming that nobody intentionally passes [null]
                    serde_json::Value::Null => {}
                    // Wrap everything else in an array
                    _ => {
                        let v = data.take();
                        *data = serde_json::Value::Array(vec![v]);
                    }
                }
            }
        }

        match schema.schema.apply(input).basic() {
            BasicOutput::Valid(_) => Ok(()),
            BasicOutput::Invalid(err) => Err(err),
        }
    })
}

fn compile_schema(input: RootSchema) -> SchemaInfo {
    let array_fields = input
        .schema
        .object
        .as_ref()
        .map(|o| {
            o.properties
                .iter()
                .filter(|(_, s)| match s {
                    schemars::schema::Schema::Object(o) => o.array.is_some(),
                    _ => false,
                })
                .map(|(k, _)| k.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let schema = match JSONSchema::compile(&serde_json::to_value(input).unwrap()) {
        Ok(schema) => schema,
        Err(_) => JSONSchema::compile(&serde_json::json!({})).unwrap(),
    };

    SchemaInfo {
        schema,
        array_fields,
    }
}
