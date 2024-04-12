//! Code for validating requests against JSON schemas
// Schema caching code in this file originally inspired by [axum-jsonschema](https://github.com/tamasfe/aide/blob/master/crates/axum-jsonschema/src/lib.rs).

use std::{
    any::TypeId,
    cell::RefCell,
    collections::{BTreeMap, HashMap, VecDeque},
};

use jsonschema::{
    output::{BasicOutput, ErrorDescription, OutputUnit},
    JSONSchema,
};
use schemars::{
    gen::{SchemaGenerator, SchemaSettings},
    schema::{InstanceType, RootSchema, SingleOrVec},
    JsonSchema,
};
use serde::Serialize;
use serde_json::Number;

thread_local! {
    static SCHEMAS: RefCell<Schemas> = RefCell::new(Schemas::new());
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
enum CoerceOp {
    #[default]
    // Don't coerce the value
    None,
    // Coerce the value's type
    Singleton,
    // Coerce the value's type and make sure it's an array
    Array,
}

impl CoerceOp {
    fn new(coerce: bool, array: bool) -> Self {
        match (coerce, array) {
            (true, true) => CoerceOp::Array,
            (true, false) => CoerceOp::Singleton,
            _ => CoerceOp::None,
        }
    }

    // Combine this CoerceOp with another. When two ops combine and one is an array, the output is
    // Singleton because that means that we don't have to coerce it into an array for validation to
    // pass.
    fn extend(&mut self, other: &CoerceOp) {
        match (&self, other) {
            (CoerceOp::None, a) => {
                *self = *a;
            }
            (CoerceOp::Singleton, _) => {}
            (CoerceOp::Array, CoerceOp::Singleton) => {
                *self = CoerceOp::Singleton;
            }
            (CoerceOp::Array, _) => {}
        }
    }

    // Create a version of this op, filtered on if array coercion is allowed in the caller's
    // context.
    fn with_array(&self, allow_array: bool) -> Self {
        match (self, allow_array) {
            (CoerceOp::Array, false) => CoerceOp::Singleton,
            _ => *self,
        }
    }

    fn is_none(&self) -> bool {
        matches!(self, CoerceOp::None)
    }

    fn is_singleton(&self) -> bool {
        matches!(self, CoerceOp::Singleton)
    }

    fn is_array(&self) -> bool {
        matches!(self, CoerceOp::Array)
    }

    fn coerce_singleton_or_array(&self, output: &mut serde_json::Value, input: serde_json::Value) {
        if self.is_array() {
            *output = serde_json::Value::Array(vec![input]);
        } else if self.is_singleton() {
            *output = input;
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CoerceTo {
    number: CoerceOp,
    integer: CoerceOp,
    boolean: CoerceOp,
    string: CoerceOp,
}

impl CoerceTo {
    fn from_instance_types(t: &SingleOrVec<InstanceType>, array: bool) -> Self {
        Self {
            number: CoerceOp::new(t.contains(&InstanceType::Number), array),
            integer: CoerceOp::new(t.contains(&InstanceType::Integer), array),
            boolean: CoerceOp::new(t.contains(&InstanceType::Boolean), array),
            string: CoerceOp::new(t.contains(&InstanceType::String), array),
        }
    }

    fn extend(&mut self, other: &CoerceTo) {
        self.number.extend(&other.number);
        self.integer.extend(&other.integer);
        self.boolean.extend(&other.boolean);
        self.string.extend(&other.string);
    }

    fn into_option(self) -> Option<Self> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }

    fn is_empty(&self) -> bool {
        self.number.is_none()
            && self.integer.is_none()
            && self.boolean.is_none()
            && !self.string.is_array()
    }

    /// Coerce from string to boolean, accounting for values that come from forms
    fn coerce_to_boolean(&self, op: CoerceOp, input: &mut serde_json::Value) {
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

                op.coerce_singleton_or_array(input, serde_json::Value::Bool(value));
            }
            _ => {}
        }
    }

    fn coerce_to_integer(&self, op: CoerceOp, input: &mut serde_json::Value) {
        match input {
            serde_json::Value::String(s) => {
                let n = s.parse::<i64>().ok();
                if let Some(number) = n {
                    op.coerce_singleton_or_array(input, serde_json::Value::Number(number.into()));
                }
            }
            _ => {}
        }
    }

    fn coerce_to_number(&self, op: CoerceOp, input: &mut serde_json::Value) {
        match input {
            serde_json::Value::String(s) => {
                if s.contains('.') {
                    let n = s.parse::<f64>().ok().and_then(Number::from_f64);
                    if let Some(number) = n {
                        op.coerce_singleton_or_array(input, serde_json::Value::Number(number));
                    }
                } else {
                    let n = s.parse::<i64>().ok();
                    if let Some(number) = n {
                        op.coerce_singleton_or_array(
                            input,
                            serde_json::Value::Number(number.into()),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn coerce(&self, input: &mut serde_json::Value, do_array_coercion: bool) {
        if !self.number.is_none() {
            self.coerce_to_number(self.number.with_array(do_array_coercion), input);
        } else if !self.integer.is_none() {
            self.coerce_to_integer(self.integer.with_array(do_array_coercion), input);
        }

        if !self.boolean.is_none() {
            self.coerce_to_boolean(self.boolean.with_array(do_array_coercion), input);
        }

        // Everything is a string already, so only do array coercion for strings.
        if do_array_coercion && input.is_string() && self.string.is_array() {
            *input = serde_json::Value::Array(vec![input.take()]);
        }
    }
}

#[derive(Debug)]
struct FieldInfo {
    name: String,
    required: bool,
    coerce: CoerceTo,
}

impl FieldInfo {
    // Browsers omit checkboxes completely if they are not checked, so supply `false` or vec![] if the
    // validator is expecting a value.
    fn coerce_none(&self) -> Option<serde_json::Value> {
        if !self.required {
            return None;
        }

        match self.coerce.boolean {
            CoerceOp::Singleton => Some(serde_json::Value::Bool(false)),
            CoerceOp::Array => Some(serde_json::Value::Array(vec![])),
            CoerceOp::None => None,
        }
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

/// Validation errors, formatted for return to the client
#[derive(Debug, Default, Serialize)]
pub struct ValidationErrorResponse {
    /// Validation messages not specific to a particular field.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<String>,
    /// Validation messages for particular fields. For nested fields, the paths are in
    /// JSON Pointer format.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, Vec<String>>,
}

impl ValidationErrorResponse {
    /// Return true if there are no validation errors
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.fields.is_empty()
    }
}

impl From<SchemaErrors> for ValidationErrorResponse {
    fn from(value: SchemaErrors) -> Self {
        let mut result = ValidationErrorResponse {
            messages: vec![],
            fields: BTreeMap::default(),
        };

        for err in value {
            let field = err.instance_location().to_string();
            let message = err.error_description().to_string();
            result.fields.entry(field).or_default().push(message);
        }

        result
    }
}

/// Validate a JSON value against a JSON schema
/// If `coerce_arrays` is true, non-array values will be coerced to `Vec<_>` if the schema
/// specifies an array. This should be used when the original input format is not completely
/// self-describing, such as `application/x-www-form-urlencoded`.
pub fn validate<T: JsonSchema + 'static>(
    mut input: serde_json::Value,
    coerce_values: bool,
) -> Result<serde_json::Value, (serde_json::Value, SchemaErrors)> {
    SCHEMAS.with(|sch| {
        let sch = &mut *sch.borrow_mut();
        let schema = sch
            .schemas
            .entry(TypeId::of::<T>())
            .or_insert_with(|| compile_schema(sch.generator.root_schema_for::<T>()));

        if coerce_values {
            // When a browser submits a form, everything is a string and there's no distinction between singletons
            // and single-element arrays. This code attempts to match the submitted data to the
            // actual format. It specifically only attempts to handle HTML form submissions, rather
            // than every possible input.
            for field in &schema.coerce_fields {
                if let Some(data) = input.get_mut(&field.name) {
                    match data {
                        serde_json::Value::Array(a) => {
                            for v in a {
                                field.coerce.coerce(v, false);
                            }
                        }
                        serde_json::Value::Null => {}
                        _ => {
                            field.coerce.coerce(data, true);
                        }
                    }
                } else if let Some(none_value) = field.coerce_none() {
                    input[field.name.clone()] = none_value;
                }
            }
        }

        match schema.schema.apply(&input).basic() {
            BasicOutput::Valid(_) => Ok(input),
            BasicOutput::Invalid(err) => Err((input, err)),
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
                .filter_map(|(name, s)| {
                    get_coerce_type::get_coerce_type_from_schema(s, false).map(|ct| FieldInfo {
                        name: name.to_string(),
                        required: root.required.contains(name),
                        coerce: ct,
                    })
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

mod get_coerce_type {
    use schemars::schema::{Schema, SchemaObject, SingleOrVec};

    use super::CoerceTo;

    /// When there are multiple subschemas for a field, combine the coercion info from them all.
    fn get_coerce_type_from_subschemas(ss: &Option<Vec<Schema>>, array: bool) -> Option<CoerceTo> {
        let Some(ss) = ss.as_deref() else {
            return None;
        };

        let mut ct = CoerceTo::default();

        for schema in ss {
            if let Some(c) = get_coerce_type_from_schema(schema, array) {
                ct.extend(&c);
            }
        }

        ct.into_option()
    }

    /// Get the schemas from an array's items.
    fn get_coerce_type_from_array_items(items: &SingleOrVec<Schema>) -> Option<CoerceTo> {
        match items {
            SingleOrVec::Single(schema) => get_coerce_type_from_schema(schema, true),
            SingleOrVec::Vec(schemas) => schemas
                .iter()
                .find_map(|s| get_coerce_type_from_schema(s, true)),
        }
    }

    /// Get the coercion info from a schema object, accounting for it using the "type" field
    /// or "anyOf" or "oneOf".
    fn get_coerce_type_from_schema_object(o: &SchemaObject, array: bool) -> Option<CoerceTo> {
        o.instance_type
            .as_ref()
            .and_then(|t| CoerceTo::from_instance_types(t, array).into_option())
            .or_else(|| {
                o.subschemas.as_ref().and_then(|ss| {
                    get_coerce_type_from_subschemas(&ss.any_of, array)
                        .or_else(|| get_coerce_type_from_subschemas(&ss.one_of, array))
                })
            })
    }

    /// Get the coercion info from a field's schema.
    pub fn get_coerce_type_from_schema(schema: &Schema, array: bool) -> Option<CoerceTo> {
        match schema {
            Schema::Object(o) => {
                if let Some(ct) = get_coerce_type_from_schema_object(o, array) {
                    Some(ct)
                } else if let Some(array) = o.array.as_ref() {
                    let ct = array
                        .items
                        .as_ref()
                        .and_then(get_coerce_type_from_array_items)
                        .unwrap_or_else(CoerceTo::default);
                    Some(ct)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
#[allow(dead_code)]
mod test {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[test]
    fn strings() {
        #[derive(Deserialize, JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            s: String,
            s_vec1: Vec<String>,
            s_vec2: Vec<String>,
            s_vec3: Vec<String>,
        }

        let data = json!({
            "s": "foo",
            "s_vec1": "foo",
            "s_vec2": "foo",
            "s_vec3": ["foo", "bar"],
        });

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();
        assert_eq!(
            data,
            json!({
                "s": "foo",
                "s_vec1": ["foo"],
                "s_vec2": ["foo"],
                "s_vec3": ["foo", "bar"],
            })
        )
    }

    #[derive(Debug, PartialEq, Eq, JsonSchema)]
    #[serde(untagged)]
    enum NumOrBool {
        Num(i32),
        Bool(bool),
    }

    #[derive(Debug, PartialEq, Eq, JsonSchema)]
    #[serde(untagged)]
    enum NumVecOrBool {
        Num(Vec<i32>),
        Bool(bool),
    }

    #[derive(Debug, PartialEq, Eq, JsonSchema)]
    #[serde(untagged)]
    enum NumOrBoolVec {
        Num(i32),
        Bool(Vec<bool>),
    }

    #[derive(Debug, PartialEq, Eq, JsonSchema)]
    #[serde(untagged)]
    enum NumVecOrBoolVec {
        Num(Vec<i32>),
        Bool(Vec<bool>),
    }

    #[test]
    fn num_or_bool() {
        #[derive(JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            nob_n: NumOrBool,
            nob_b: NumOrBool,
            nob_b_option: Option<NumOrBool>,
            nob_b_omitted: NumOrBool,
            nob_v1: Vec<NumOrBool>,
            nob_v2: Vec<NumOrBool>,
            nob_v3: Vec<NumOrBool>,
        }

        let data = json!({
            "nob_n": "1",
            "nob_b": "on",
            "nob_v1": "1",
            "nob_v2": "on",
            "nob_v3": ["on", 1, 23],
        });

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();
        assert_eq!(
            data,
            json!({
                "nob_n": 1,
                "nob_b": true,
                "nob_b_omitted": false,
                "nob_v1": [1],
                "nob_v2": [true],
                "nob_v3": [true, 1, 23],
            })
        )
    }

    #[test]
    fn num_vec_or_bool_vec() {
        #[derive(JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            b1: NumVecOrBoolVec,
            n1: NumVecOrBoolVec,
            b2: NumVecOrBoolVec,
            n2: NumVecOrBoolVec,
        }

        let data = json!({
            "b1": "on",
            "n1": "1",
            "b2": ["on", "on"],
            "n2": ["1", "2"],
        });

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();
        assert_eq!(
            data,
            json!({
                "b1": [true],
                "n1": [1],
                "b2": [true, true],
                "n2": [1, 2],
            })
        );
    }

    #[test]
    fn num_or_bool_vec() {
        #[derive(JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            n: NumOrBoolVec,
            b1: NumOrBoolVec,
            b2: NumOrBoolVec,
        }

        let data = json!({
            "n": "1",
            "b1": "on",
            "b2": ["on", "on"],
        });

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();
        assert_eq!(
            data,
            json!({
                "n": 1,
                "b1": [true],
                "b2": [true, true],
            })
        );
    }

    #[test]
    fn coerce_none() {
        #[derive(JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            b: bool,
            bv: Vec<bool>,
        }

        let data = json!({});

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();
        assert_eq!(
            data,
            json!({
                "b": false,
                "bv": [],
            })
        );
    }

    #[test]
    fn comprehensive_coerce() {
        #[derive(JsonSchema, Debug, PartialEq, Eq)]
        struct Data {
            s: String,
            s_vec1: Vec<String>,
            s_vec2: Vec<String>,
            i: i32,
            i_vec1: Vec<i32>,
            i_vec2: Vec<i32>,
            nob_n: NumOrBool,
            nob_b: NumOrBool,
            nob_vec1: Vec<NumOrBool>,
            nob_vec2: Vec<NumOrBool>,
            nobv1: NumOrBoolVec,
            nobv2: NumOrBoolVec,
            nvob1: NumVecOrBool,
            nvob2: NumVecOrBool,
            nvobv1: NumVecOrBoolVec,
            nvobv2: NumVecOrBoolVec,
            b: bool,
            b_omitted: bool,
            ob: Option<bool>,
            b_vec1: Vec<bool>,
            b_vec2: Vec<bool>,
        }

        let data = json!({
            "s": "a",
            "s_vec1": "a",
            "s_vec2": ["a", "b"],
            "i": "1",
            "i_vec1": "1",
            "i_vec2": ["1", "2"],
            "nob_n": "1",
            "nob_b": "on",
            "nob_vec1": "1",
            "nob_vec2": ["1", "on"],
            "nobv1": "1",
            "nobv2": "on",
            "nvob1": "1",
            "nvob2": "on",
            "nvobv1": "1",
            "nvobv2": "on",
            "b": "on",
            "b_vec1": "on",
            "b_vec2": ["on", "false"]
        });

        let res = validate::<Data>(data, true);
        println!("{res:#?}");
        let data = res.unwrap();

        assert_eq!(
            data,
            json!({
                "s": "a",
                "s_vec1": ["a"],
                "s_vec2": ["a", "b"],
                "i": 1,
                "i_vec1": [1],
                "i_vec2": [1, 2],
                "nob_n": 1,
                "nob_b": true,
                "nob_vec1": [1],
                "nob_vec2": [1, true],
                "nobv1": 1,
                "nobv2": [true],
                "nvob1": [1],
                "nvob2": true,
                "nvobv1": [1],
                "nvobv2": [true],
                "b": true,
                "b_omitted": false,
                "b_vec1": [true],
                "b_vec2": [true, false]
            })
        );
    }
}
