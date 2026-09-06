pub(crate) fn quoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => output.extend(character.escape_default()),
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

pub(crate) fn option_quoted(value: Option<&str>) -> String {
    value.map_or_else(|| "None".to_string(), quoted)
}
