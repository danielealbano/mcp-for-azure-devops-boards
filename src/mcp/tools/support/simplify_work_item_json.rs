use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;

// Static regex patterns for text cleaning (compiled once, reused many times)
static RE_SPACES: Lazy<Regex> = Lazy::new(|| Regex::new(r" +").unwrap());
static RE_NEWLINES: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n+").unwrap());
static RE_LEADING_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n[ ]+").unwrap());
static RE_TRAILING_WS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ ]+\n").unwrap());
static RE_DASHES: Lazy<Regex> = Lazy::new(|| Regex::new(r"-{3,}\n").unwrap());
static RE_IMAGE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\[image\]").unwrap());

/// Recursively simplifies the JSON output to reduce token usage for LLMs.
/// It removes "_links", "url", "descriptor", "imageUrl", "avatar" and simplifies field names.
/// It also flattens the "fields" object to the root level and removes redundant properties.
pub fn simplify_work_item_json(value: &mut Value) {
    match value {
        Value::Object(map) => {
            // Remove unnecessary fields at the top level and in nested objects
            map.remove("url");
            map.remove("_links");
            map.remove("descriptor");
            map.remove("imageUrl");
            map.remove("avatar");

            // Process "fields" if present (specific to Work Items)
            if let Some(Value::Object(mut fields_map)) = map.remove("fields") {
                let mut simplified_fields = serde_json::Map::new();

                // Collect keys to process
                let keys: Vec<String> = fields_map.keys().cloned().collect();

                for key in keys {
                    if let Some(mut val) = fields_map.remove(&key) {
                        // Simplify Identity fields (objects with displayName, uniqueName, etc.)
                        if let Value::Object(ref obj) = val
                            && let Some(Value::String(name)) = obj.get("displayName")
                        {
                            let mut display_value = name.clone();
                            if let Some(Value::String(unique_name)) = obj.get("uniqueName")
                                && !unique_name.is_empty()
                            {
                                display_value = format!("{} <{}>", name, unique_name);
                            }
                            val = Value::String(display_value);
                        }

                        // Simplify field names and filter out unwanted fields
                        let new_key = if key.starts_with("System.") {
                            key.strip_prefix("System.").unwrap().to_string()
                        } else if key.starts_with("Microsoft.VSTS.Common.") {
                            key.strip_prefix("Microsoft.VSTS.Common.")
                                .unwrap()
                                .to_string()
                        } else if key.starts_with("Microsoft.VSTS.Scheduling.") {
                            key.strip_prefix("Microsoft.VSTS.Scheduling.")
                                .unwrap()
                                .to_string()
                        } else if key.starts_with("Microsoft.VSTS.CMMI.") {
                            key.strip_prefix("Microsoft.VSTS.CMMI.")
                                .unwrap()
                                .to_string()
                        } else if key.contains("_Kanban.Column") {
                            // Handle dynamic WEF_..._Kanban.Column -> Column
                            "Column".to_string()
                        } else if key.contains("_Kanban.Lane") {
                            // Handle dynamic WEF_..._Kanban.Lane -> Lane
                            "Lane".to_string()
                        } else {
                            key
                        };

                        // Skip unwanted fields
                        if matches!(
                            new_key.as_str(),
                            "ActivatedBy"
                                | "ActivatedDate"
                                | "BoardColumnDone"
                                | "ClosedBy"
                                | "ClosedDate"
                                | "Column.Done"
                                | "CommentCount"
                                | "Reason"
                                | "ResolvedBy"
                                | "ResolvedDate"
                                | "State"
                        ) {
                            continue;
                        }

                        // Rename BoardColumn to Column and BoardLane to Lane
                        let final_key = match new_key.as_str() {
                            "BoardColumn" => "Column".to_string(),
                            "BoardLane" => "Lane".to_string(),
                            "AcceptanceCriteria" => "Acceptance".to_string(),
                            "TeamProject" => "Project".to_string(),
                            "WorkItemType" => "Type".to_string(),
                            "IterationPath" => "Iteration".to_string(),
                            _ => new_key,
                        };

                        // Convert HTML to text for specific fields
                        if matches!(
                            final_key.as_str(),
                            "Acceptance" | "Description" | "Justification"
                        ) {
                            if let Value::String(html_content) = &val {
                                // Convert HTML to plain text, width doesn't matter as we don't need wrapping
                                if let Ok(mut plain_text) =
                                    html2text::from_read(html_content.as_bytes(), usize::MAX)
                                {
                                    // Normalize newlines: replace \r with \n
                                    plain_text = plain_text.replace('\r', "\n");

                                    // Normalize tabulations: replace \t with 1 space
                                    plain_text = plain_text.replace('\t', " ");

                                    // Normalize emdashes: replace ─ with -
                                    plain_text = plain_text.replace('─', "-");

                                    // Remove multiple consecutive spaces
                                    plain_text =
                                        RE_SPACES.replace_all(&plain_text, " ").to_string();

                                    // Collapse multiple consecutive newlines into single newlines
                                    plain_text =
                                        RE_NEWLINES.replace_all(&plain_text, "\n").to_string();

                                    // Remove leading whitespace before newlines (spaces, tabs, etc.)
                                    plain_text =
                                        RE_LEADING_WS.replace_all(&plain_text, "\n").to_string();

                                    // Remove trailing whitespace before newlines (spaces, tabs, etc.)
                                    plain_text =
                                        RE_TRAILING_WS.replace_all(&plain_text, "\n").to_string();

                                    // Collapse 3+ dashes followed by newline to just 3 dashes + newline
                                    plain_text =
                                        RE_DASHES.replace_all(&plain_text, "---\n").to_string();

                                    // Remove [Image] strings (case insensitive)
                                    plain_text = RE_IMAGE.replace_all(&plain_text, "").to_string();

                                    val = Value::String(plain_text.trim().to_string());
                                }
                            }
                        }

                        // Optimize Tags field by removing spaces after semicolons
                        if final_key == "Tags" {
                            if let Value::String(tags) = &val {
                                val = Value::String(tags.replace("; ", ";"));
                            }
                        }

                        // Abbreviate Type field to just first letter
                        if final_key == "Type" {
                            if let Value::String(type_val) = &val {
                                if let Some(first_char) = type_val.chars().next() {
                                    val = Value::String(first_char.to_string());
                                }
                            }
                        }

                        // Only insert if not already present (prefer existing values)
                        if !simplified_fields.contains_key(&final_key) {
                            simplified_fields.insert(final_key, val);
                        }
                    }
                }

                // Flatten: move all simplified fields to the root level
                for (k, v) in simplified_fields {
                    map.insert(k, v);
                }
            }

            // Recursively process all remaining values
            for (_, v) in map.iter_mut() {
                simplify_work_item_json(v);
            }
        }
        Value::Array(arr) => {
            // Recursively process all array elements
            for item in arr.iter_mut() {
                simplify_work_item_json(item);
            }
        }
        _ => {}
    }
}
