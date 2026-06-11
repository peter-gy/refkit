use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, TextDecoration, VerticalAlign,
};
use hayagriva::types::EntryType;
use hayagriva::{ElemMeta, Formatting};

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

pub(crate) fn option_quoted(value: Option<&str>) -> String {
    value.map_or_else(|| "None".to_string(), quoted)
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

pub(crate) fn display_name(display: Display) -> &'static str {
    match display {
        Display::Block => "Block",
        Display::LeftMargin => "LeftMargin",
        Display::RightInline => "RightInline",
        Display::Indent => "Indent",
    }
}

pub(crate) fn elem_meta_name(meta: &ElemMeta) -> &'static str {
    match meta {
        ElemMeta::Names(_) => "Names",
        ElemMeta::Date => "Date",
        ElemMeta::Text => "Text",
        ElemMeta::Number => "Number",
        ElemMeta::Label => "Label",
        ElemMeta::CitationNumber => "CitationNumber",
        ElemMeta::Name(_, _) => "Name",
        ElemMeta::Entry(_) => "Entry",
        ElemMeta::CitationLabel => "CitationLabel",
    }
}

pub(crate) fn font_style_name(font_style: FontStyle) -> &'static str {
    match font_style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
    }
}

pub(crate) fn font_variant_name(font_variant: FontVariant) -> &'static str {
    match font_variant {
        FontVariant::Normal => "Normal",
        FontVariant::SmallCaps => "SmallCaps",
    }
}

pub(crate) fn font_weight_name(font_weight: FontWeight) -> &'static str {
    match font_weight {
        FontWeight::Normal => "Normal",
        FontWeight::Bold => "Bold",
        FontWeight::Light => "Light",
    }
}

pub(crate) fn text_decoration_name(text_decoration: TextDecoration) -> &'static str {
    match text_decoration {
        TextDecoration::None => "None",
        TextDecoration::Underline => "Underline",
    }
}

pub(crate) fn vertical_align_name(vertical_align: VerticalAlign) -> &'static str {
    match vertical_align {
        VerticalAlign::None => "None",
        VerticalAlign::Baseline => "Baseline",
        VerticalAlign::Sup => "Sup",
        VerticalAlign::Sub => "Sub",
    }
}

pub(crate) fn formatting_summary(formatting: Formatting) -> String {
    if formatting == Formatting::default() {
        return "Normal".to_string();
    }

    [
        ("font_style", font_style_name(formatting.font_style)),
        ("font_variant", font_variant_name(formatting.font_variant)),
        ("font_weight", font_weight_name(formatting.font_weight)),
        (
            "text_decoration",
            text_decoration_name(formatting.text_decoration),
        ),
        (
            "vertical_align",
            vertical_align_name(formatting.vertical_align),
        ),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={value}"))
    .collect::<Vec<_>>()
    .join(",")
}

#[cfg(test)]
mod tests {
    use hayagriva::citationberg::taxonomy::NameVariable;

    use super::*;

    #[test]
    fn entry_type_names_cover_public_hayagriva_types() {
        assert_eq!(entry_type_name(&EntryType::Article), "Article");
        assert_eq!(entry_type_name(&EntryType::Periodical), "Periodical");
        assert_eq!(entry_type_name(&EntryType::Original), "Original");
    }

    #[test]
    fn quoted_strings_are_stable_python_boundary_strings() {
        assert_eq!(quoted("doe2024"), "\"doe2024\"");
        assert_eq!(quoted("O'Reilly\\n"), "\"O'Reilly\\\\n\"");
        assert_eq!(quoted("line\nbreak"), "\"line\\nbreak\"");
        assert_eq!(quoted("quote\""), "\"quote\\\"\"");
        assert_eq!(option_quoted(Some("page")), "\"page\"");
        assert_eq!(option_quoted(None), "None");
    }

    #[test]
    fn rendered_public_names_do_not_depend_on_debug_payloads() {
        assert_eq!(display_name(Display::LeftMargin), "LeftMargin");
        assert_eq!(elem_meta_name(&ElemMeta::Entry(42)), "Entry");
        assert_eq!(
            elem_meta_name(&ElemMeta::Name(NameVariable::Author, 0)),
            "Name"
        );
        assert_eq!(font_style_name(FontStyle::Italic), "Italic");
        assert_eq!(font_variant_name(FontVariant::SmallCaps), "SmallCaps");
        assert_eq!(font_weight_name(FontWeight::Bold), "Bold");
        assert_eq!(text_decoration_name(TextDecoration::Underline), "Underline");
        assert_eq!(vertical_align_name(VerticalAlign::Sup), "Sup");
        assert_eq!(formatting_summary(Formatting::default()), "Normal");
    }
}
