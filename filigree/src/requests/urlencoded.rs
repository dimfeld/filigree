use form_urlencoded;

/// Create a [serde_json::Value] from a form-urlencoded string
pub fn value_from_urlencoded(v: &[u8]) -> serde_json::Value {
    let mut output = serde_json::value::Map::<String, serde_json::Value>::new();

    for (key, val) in form_urlencoded::parse(v).into_owned() {
        if let Some(entry) = output.get_mut(&key) {
            if let serde_json::Value::Array(a) = entry {
                a.push(serde_json::Value::String(val.to_string()));
            } else {
                let existing = entry.take();
                *entry = serde_json::Value::Array(vec![
                    existing,
                    serde_json::Value::String(val.to_string()),
                ]);
            }
        } else {
            output.insert(key.to_string(), serde_json::Value::String(val.to_string()));
        }
    }

    serde_json::Value::Object(output)
}
