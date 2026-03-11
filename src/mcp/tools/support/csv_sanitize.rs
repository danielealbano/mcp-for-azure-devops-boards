pub fn sanitize_csv_value(s: &str) -> String {
    let escaped = s
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "");
    if escaped.starts_with('=')
        || escaped.starts_with('+')
        || escaped.starts_with('-')
        || escaped.starts_with('@')
    {
        format!("'{}", escaped)
    } else {
        escaped
    }
}
