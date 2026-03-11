use serde::Serialize;

/// Serializes a value to a compact representation optimized for LLM consumption.
/// This format removes all unnecessary whitespace and quotes, only escaping newlines.
///
/// Example output: {id:123,name:John Doe,tags:[tag1,tag2],active:true}
pub fn to_compact_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    // First serialize to standard JSON to get the structure
    let json_value = serde_json::to_value(value)?;

    // Then convert to compact format
    let mut output = String::new();
    write_compact_value(&json_value, &mut output);

    Ok(output)
}

const MAX_RECURSION_DEPTH: usize = 64;

fn write_compact_value(value: &serde_json::Value, output: &mut String) {
    write_compact_value_inner(value, output, 0);
}

fn write_compact_value_inner(value: &serde_json::Value, output: &mut String, depth: usize) {
    if depth > MAX_RECURSION_DEPTH {
        output.push_str("...");
        return;
    }
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        serde_json::Value::Number(n) => output.push_str(&n.to_string()),
        serde_json::Value::String(s) => {
            let escaped = s.replace('\n', "\\n").replace('\r', "\\r");
            output.push_str(&escaped);
        }
        serde_json::Value::Array(arr) => {
            output.push('[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                write_compact_value_inner(item, output, depth + 1);
            }
            output.push(']');
        }
        serde_json::Value::Object(obj) => {
            output.push('{');
            for (i, (key, val)) in obj.iter().enumerate() {
                if i > 0 {
                    output.push(',');
                }
                let escaped_key = key.replace('\n', "\\n").replace('\r', "\\r");
                output.push_str(&escaped_key);
                output.push(':');
                write_compact_value_inner(val, output, depth + 1);
            }
            output.push('}');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestStruct {
        name: String,
        age: u32,
        active: bool,
        tags: Vec<String>,
    }

    #[test]
    fn test_basic_serialization() {
        let data = TestStruct {
            name: "John Doe".to_string(),
            age: 30,
            active: true,
            tags: vec!["rust".to_string(), "developer".to_string()],
        };

        let result = to_compact_string(&data).unwrap();
        // JSON object keys are not ordered, so check for key presence
        assert!(result.contains("name:John Doe"));
        assert!(result.contains("age:30"));
        assert!(result.contains("active:true"));
        assert!(result.contains("tags:[rust,developer]"));
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn test_newline_escaping() {
        #[derive(Serialize)]
        struct WithNewlines {
            text: String,
        }

        let data = WithNewlines {
            text: "Line 1\nLine 2\rLine 3".to_string(),
        };

        let result = to_compact_string(&data).unwrap();
        assert_eq!(result, "{text:Line 1\\nLine 2\\rLine 3}");
    }

    #[test]
    fn test_nested_objects() {
        use serde_json::json;

        let data = json!({
            "user": {
                "id": 123,
                "name": "Alice"
            },
            "items": [1, 2, 3]
        });

        let result = to_compact_string(&data).unwrap();
        // Check for key presence since order is not guaranteed
        assert!(result.contains("items:[1,2,3]"));
        assert!(result.contains("user:{"));
        assert!(result.contains("id:123"));
        assert!(result.contains("name:Alice"));
    }

    #[test]
    fn test_special_values() {
        use serde_json::json;

        let data = json!({
            "null_value": null,
            "bool_true": true,
            "bool_false": false,
            "number": 42.5
        });

        let result = to_compact_string(&data).unwrap();
        assert!(result.contains("null_value:null"));
        assert!(result.contains("bool_true:true"));
        assert!(result.contains("bool_false:false"));
        assert!(result.contains("number:42.5"));
    }

    #[test]
    fn test_empty_object() {
        use serde_json::json;
        let data = json!({});
        let result = to_compact_string(&data).unwrap();
        assert_eq!(result, "{}");
    }

    #[test]
    fn test_empty_array() {
        let data: Vec<String> = vec![];
        let result = to_compact_string(&data).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_unicode_strings() {
        use serde_json::json;
        let data = json!({"cjk": "日本語", "emoji": "🦀🔥", "accented": "café résumé"});
        let result = to_compact_string(&data).unwrap();
        assert!(result.contains("cjk:日本語"));
        assert!(result.contains("emoji:🦀🔥"));
        assert!(result.contains("accented:café résumé"));
    }

    #[test]
    fn test_control_characters() {
        use serde_json::json;
        let data = json!({"tab": "a\tb", "null": "a\0b"});
        let result = to_compact_string(&data).unwrap();
        assert!(result.contains("tab:a\tb"));
        assert!(result.contains("null:a\0b"));
    }

    #[test]
    fn test_deeply_nested_object() {
        use serde_json::json;
        let mut value = json!("leaf");
        for _ in 0..64 {
            value = json!({"n": value});
        }
        let result = to_compact_string(&value).unwrap();
        assert!(result.contains("leaf"));
        assert!(!result.contains("..."));
    }

    #[test]
    fn test_beyond_max_depth() {
        use serde_json::json;
        let mut value = json!("leaf");
        for _ in 0..66 {
            value = json!({"n": value});
        }
        let result = to_compact_string(&value).unwrap();
        assert!(result.contains("..."));
    }

    #[test]
    fn test_long_string() {
        let long = "a".repeat(10000);
        let result = to_compact_string(&long).unwrap();
        assert_eq!(result.len(), 10000);
    }

    #[test]
    fn test_empty_string_value() {
        use serde_json::json;
        let data = json!({"key": ""});
        let result = to_compact_string(&data).unwrap();
        assert_eq!(result, "{key:}");
    }

    #[test]
    fn test_mixed_array() {
        use serde_json::json;
        let data = json!([null, true, 42, "hello", {"k": "v"}]);
        let result = to_compact_string(&data).unwrap();
        assert_eq!(result, "[null,true,42,hello,{k:v}]");
    }
}
