use crate::compact_llm;
use serde_json::Value;

/// Converts work items JSON to CSV format with dynamic column detection.
/// Only includes columns that have at least one non-null value across all items.
pub fn work_items_to_csv(json_value: &Value) -> Result<String, String> {
    // Define all possible fields in preferred order
    let all_fields = vec![
        "id",
        "Type",
        "Title",
        "Description",
        "Acceptance",
        "Column",
        "Lane",
        "Priority",
        "AssignedTo",
        "CreatedBy",
        "CreatedDate",
        "ChangedBy",
        "ChangedDate",
        "AreaPath",
        "Iteration",
        "Project",
        "Tags",
        "StartDate",
        "TargetDate",
        "Effort",
        "Risk",
        "Justification",
        "ValueArea",
        "StackRank",
        "StateChangeDate",
        "History",
        "comments",
    ];

    // Normalize input to array
    let items = match json_value {
        Value::Array(arr) => arr.as_slice(),
        Value::Object(_) => std::slice::from_ref(json_value),
        _ => return Err("Invalid input: expected object or array".to_string()),
    };

    if items.is_empty() {
        return Ok(String::new());
    }

    // Detect which fields actually have values
    let mut active_fields = Vec::new();
    for field in &all_fields {
        let has_value = items.iter().any(|item| {
            item.get(field)
                .map(|v| !v.is_null() && v.as_str().map_or(true, |s| !s.is_empty()))
                .unwrap_or(false)
        });
        if has_value {
            active_fields.push(*field);
        }
    }

    // Build CSV
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write header
    wtr.write_record(&active_fields)
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write rows
    for item in items {
        let row: Vec<String> = active_fields
            .iter()
            .map(|field| {
                item.get(*field)
                    .and_then(|v| match v {
                        Value::String(s) => {
                            // Escape newlines and tabs for better LLM consumption
                            let escaped = s
                                .replace('\n', "\\n")
                                .replace('\t', "\\t")
                                .replace('\r', ""); // Remove carriage returns entirely
                            Some(escaped)
                        }
                        Value::Number(n) => Some(n.to_string()),
                        Value::Bool(b) => Some(b.to_string()),
                        Value::Array(_) if *field == "comments" => {
                            // Serialize comments array as compact JSON using compact_llm
                            compact_llm::to_compact_string(v).ok()
                        }
                        _ => None,
                    })
                    .unwrap_or_default()
            })
            .collect();

        wtr.write_record(&row)
            .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;

    let csv_bytes = wtr
        .into_inner()
        .map_err(|e| format!("Failed to get CSV bytes: {}", e))?;

    String::from_utf8(csv_bytes).map_err(|e| format!("Failed to convert CSV to string: {}", e))
}
