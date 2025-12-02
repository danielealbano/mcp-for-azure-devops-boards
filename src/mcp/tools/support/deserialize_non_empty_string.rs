use serde::Deserialize;

/// Custom deserializer for non-empty strings
pub fn deserialize_non_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.trim().is_empty() {
        return Err(serde::de::Error::custom("field cannot be empty"));
    }
    Ok(s.trim().to_string())
}
