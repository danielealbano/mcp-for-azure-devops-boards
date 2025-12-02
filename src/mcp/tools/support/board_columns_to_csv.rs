use crate::azure::boards;

/// Converts board columns to CSV format.
/// Columns: name, item_limit (WIP), is_split, column_type
pub fn board_columns_to_csv(columns: &[boards::BoardColumn]) -> Result<String, String> {
    let mut wtr = csv::Writer::from_writer(vec![]);

    // Write header
    wtr.write_record(&["name", "item_limit", "is_split", "column_type"])
        .map_err(|e| format!("Failed to write CSV header: {}", e))?;

    // Write rows
    for column in columns {
        wtr.write_record(&[
            &column.name,
            &column.item_limit.to_string(),
            &column.is_split.unwrap_or(false).to_string(),
            &column.column_type,
        ])
        .map_err(|e| format!("Failed to write CSV row: {}", e))?;
    }

    wtr.flush()
        .map_err(|e| format!("Failed to flush CSV writer: {}", e))?;

    let csv_bytes = wtr
        .into_inner()
        .map_err(|e| format!("Failed to get CSV bytes: {}", e))?;

    String::from_utf8(csv_bytes).map_err(|e| format!("Failed to convert CSV to string: {}", e))
}
