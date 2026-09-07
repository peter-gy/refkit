use hayagriva::types::EntryType;

pub(crate) fn quoted(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.extend(ch.escape_default()),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}

pub(crate) fn entry_type_name(entry_type: &EntryType) -> &'static str {
    match entry_type {
        EntryType::Article => "Article",
        EntryType::Chapter => "Chapter",
        EntryType::Entry => "Entry",
        EntryType::Anthos => "Anthos",
        EntryType::Report => "Report",
        EntryType::Thesis => "Thesis",
        EntryType::Web => "Web",
        EntryType::Scene => "Scene",
        EntryType::Artwork => "Artwork",
        EntryType::Patent => "Patent",
        EntryType::Case => "Case",
        EntryType::Newspaper => "Newspaper",
        EntryType::Legislation => "Legislation",
        EntryType::Manuscript => "Manuscript",
        EntryType::Post => "Post",
        EntryType::Misc => "Misc",
        EntryType::Performance => "Performance",
        EntryType::Periodical => "Periodical",
        EntryType::Proceedings => "Proceedings",
        EntryType::Book => "Book",
        EntryType::Blog => "Blog",
        EntryType::Reference => "Reference",
        EntryType::Conference => "Conference",
        EntryType::Anthology => "Anthology",
        EntryType::Repository => "Repository",
        EntryType::Thread => "Thread",
        EntryType::Video => "Video",
        EntryType::Audio => "Audio",
        EntryType::Exhibition => "Exhibition",
        EntryType::Original => "Original",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_type_names_cover_public_hayagriva_types() {
        assert_eq!(entry_type_name(&EntryType::Article), "Article");
        assert_eq!(entry_type_name(&EntryType::Periodical), "Periodical");
        assert_eq!(entry_type_name(&EntryType::Original), "Original");
    }

    #[test]
    fn quoted_strings_escape_diagnostic_values() {
        assert_eq!(quoted("doe2024"), "\"doe2024\"");
        assert_eq!(quoted("O'Reilly\\n"), "\"O'Reilly\\\\n\"");
        assert_eq!(quoted("line\nbreak"), "\"line\\nbreak\"");
        assert_eq!(quoted("quote\""), "\"quote\\\"\"");
    }
}
