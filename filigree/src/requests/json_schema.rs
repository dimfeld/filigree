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
    schema::{InstanceType, RootSchema, Schema, SingleOrVec},
    JsonSchema,
};

thread_local! {
    static SCHEMAS: RefCell<Schemas> = RefCell::new(Schemas::new());
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CoerceTo {
    number: bool,
    boolean: bool,
}

impl CoerceTo {
    fn from_instance_types(t: &SingleOrVec<InstanceType>) -> Self {
        Self {
            number: t.contains(&InstanceType::Number),
            boolean: t.contains(&InstanceType::Boolean),
        }
    }

    fn into_option(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn is_empty(&self) -> bool {
        !(self.number || self.boolean)
    }

    /// Try to coerce a string value to a number
    fn coerce_to_number(&self, input: &mut serde_json::Value) {
        match input {
            serde_json::Value::String(s) => {
                if s.contains('.') {
                    let n = s.parse::<f64>().ok().and_then(serde_json::Number::from_f64);
                    if let Some(number) = n {
                        *input = serde_json::Value::Number(number);
                    }
                } else {
                    let n = s.parse::<i64>().ok();
                    if let Some(number) = n {
                        *input = serde_json::Value::Number(number.into());
                    }
                }
            }
            _ => {}
        }
    }

    /// Coerce from string to boolean, accounting for values that come from forms
    fn coerce_to_boolean(&self, input: &mut serde_json::Value) {
        match input {
            serde_json::Value::String(s) => {
                let value = match s.as_str() {
                    "true" => true,
                    "on" => true,
                    "false" => false,
                    "" => false,
                    _ => {
                        return;
                    }
                };

                *input = serde_json::Value::Bool(value);
            }
            _ => {}
        }
    }

    fn coerce(&self, input: &mut serde_json::Value) {
        if self.number {
            self.coerce_to_number(input);
        }

        if self.boolean {
            self.coerce_to_boolean(input);
        }
    }
}

struct FieldInfo {
    name: String,
    required: bool,
    array: bool,
    coerce: CoerceTo,
}

impl FieldInfo {
    fn coerce_none(&self) -> Option<serde_json::Value> {
        if !self.required {
            return None;
        }

        self.coerce
            .boolean
            .then_some(serde_json::Value::Bool(false))
    }
}

struct SchemaInfo {
    schema: JSONSchema,
    coerce_fields: Vec<FieldInfo>,
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

/// Errors returned from JSON schema validation
pub type SchemaErrors = VecDeque<OutputUnit<ErrorDescription>>;

/// Validate a JSON value against a JSON schema
/// If `coerce_arrays` is true, non-array values will be coerced to `Vec<_>` if the schema
/// specifies an array. This should be used when the original input format is not completely
/// self-describing, such as `application/x-www-form-urlencoded`.
pub fn validate<T: JsonSchema + 'static>(
    input: &mut serde_json::Value,
    coerce_values: bool,
) -> Result<(), SchemaErrors> {
    SCHEMAS.with(|sch| {
        let sch = &mut *sch.borrow_mut();
        let schema = sch
            .schemas
            .entry(TypeId::of::<T>())
            .or_insert_with(|| compile_schema(sch.generator.root_schema_for::<T>()));

        if coerce_values {
            // The format this was serialized from can't distinguish between singletons and single element
            // arrays, so coerce them according to the schema.
            for field in &schema.coerce_fields {
                if let Some(data) = input.get_mut(&field.name) {
                    match data {
                        serde_json::Value::Array(a) => {
                            for v in a {
                                field.coerce.coerce(v);
                            }
                        }
                        serde_json::Value::Null => {}
                        // Wrap everything else in an array
                        _ => {
                            field.coerce.coerce(data);
                            if field.array {
                                let v = data.take();
                                *data = serde_json::Value::Array(vec![v]);
                            }
                        }
                    }
                } else if let Some(none_value) = field.coerce_none() {
                    input[field.name.clone()] = none_value;
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
    let coerce_fields = input
        .schema
        .object
        .as_ref()
        .map(|root| {
            root.properties
                .iter()
                .filter_map(|(name, s)| match s {
                    Schema::Object(o) => {
                        let required = root.required.contains(name);
                        let (is_array, coerce_to) = o
                            .instance_type
                            .as_ref()
                            .map(|t| {
                                let string = t.contains(&InstanceType::String);
                                if string {
                                    return (false, CoerceTo::default());
                                }
                                let c = CoerceTo::from_instance_types(t);
                                if !c.is_empty() || !t.contains(&InstanceType::Array) {
                                    (false, c)
                                } else {
                                    let coerce_to = o
                                        .array
                                        .as_ref()
                                        .and_then(|a| a.items.as_ref())
                                        .and_then(get_coerce_type_from_array_items)
                                        .unwrap_or_else(CoerceTo::default);
                                    (true, coerce_to)
                                }
                            })
                            .unwrap_or((false, CoerceTo::default()));

                        if is_array || !coerce_to.is_empty() {
                            Some(FieldInfo {
                                name: name.to_string(),
                                required,
                                array: is_array,
                                coerce: coerce_to,
                            })
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let schema = match JSONSchema::compile(&serde_json::to_value(input).unwrap()) {
        Ok(schema) => schema,
        Err(_) => JSONSchema::compile(&serde_json::json!({})).unwrap(),
    };

    SchemaInfo {
        schema,
        coerce_fields,
    }
}

fn get_coerce_type_from_array_items(items: &SingleOrVec<Schema>) -> Option<CoerceTo> {
    match items {
        SingleOrVec::Single(schema) => get_coerce_type_from_schema(schema),
        SingleOrVec::Vec(schemas) => schemas.iter().find_map(get_coerce_type_from_schema),
    }
}

fn get_coerce_type_from_schema(schema: &Schema) -> Option<CoerceTo> {
    match schema {
        Schema::Object(o) => o.instance_type.as_ref().and_then(|t| {
            if t.contains(&InstanceType::String) {
                return None;
            }

            CoerceTo::from_instance_types(t).into_option()
        }),
        _ => None,
    }
}

#[cfg(test)]
mod test {}
