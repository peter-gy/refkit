use crate::{RawSyntaxField, RawValueAtom, RawValueMode};

use super::{render_field_name, separator};
use crate::tidy::{
    TidyOptions,
    latex::{double_enclose, flatten_unprotected_braces, strip_simple_outer_brace},
    unicode::latex_escape,
};

pub(super) fn render_value(field: &RawSyntaxField, options: &TidyOptions) -> String {
    if field.value_mode == RawValueMode::Expression {
        return render_expression_atoms(&field.value_atoms);
    }

    let (value, original_mode) = transformed_value(field, options);

    let value_mode = if options.months && field.name.eq_ignore_ascii_case("month") {
        original_mode
    } else if options.numeric && is_numeric_value(&value) {
        RawValueMode::Bare
    } else if options.curly && !is_bare_month(field, &value) {
        RawValueMode::Braced
    } else {
        original_mode
    };
    let value = if value_mode == RawValueMode::Braced {
        wrap_braced_value(field, &value, options)
    } else {
        value
    };

    match value_mode {
        RawValueMode::Bare | RawValueMode::Missing => value,
        RawValueMode::Braced | RawValueMode::Expression => format!("{{{value}}}"),
        RawValueMode::Quoted => format!("\"{value}\""),
    }
}

fn render_expression_atoms(atoms: &[RawValueAtom]) -> String {
    atoms
        .iter()
        .map(render_expression_atom)
        .filter(|atom| !atom.is_empty())
        .collect::<Vec<_>>()
        .join(" # ")
}

fn render_expression_atom(atom: &RawValueAtom) -> String {
    match atom.value_mode {
        RawValueMode::Bare | RawValueMode::Missing | RawValueMode::Expression => {
            atom.value.trim().to_string()
        }
        RawValueMode::Braced => format!("{{{}}}", atom.value),
        RawValueMode::Quoted => format!("\"{}\"", atom.value),
    }
}

fn transformed_value(field: &RawSyntaxField, options: &TidyOptions) -> (String, RawValueMode) {
    if options.months
        && field.name.eq_ignore_ascii_case("month")
        && let Some(month) = abbreviate_month(&field.value)
    {
        return (month.to_string(), RawValueMode::Bare);
    }

    let mut value = field.value.clone();

    if options.drop_all_caps {
        value = drop_all_caps(&value);
    }

    if options.encode_urls && field.name.eq_ignore_ascii_case("url") {
        value = encode_url(&value);
    }

    if options.escape && !is_verbatim_field(&field.name) {
        value = escape_characters(&value);
    }

    if field.name.eq_ignore_ascii_case("pages") {
        value = format_page_range(&value);
    }

    if field.name.eq_ignore_ascii_case("author")
        && let Some(max_authors) = options.max_authors
    {
        value = limit_authors(&value, max_authors);
    }

    if options.strip_enclosing_braces {
        value = strip_simple_outer_brace(&value);
    }

    if should_transform_field(&field.name, options.remove_braces.as_deref()) {
        value = flatten_unprotected_braces(&value);
    }

    if should_transform_field(&field.name, options.enclosing_braces.as_deref()) {
        value = double_enclose(&value);
    }

    if field.value_mode == RawValueMode::Braced {
        value = unwrap_text(&value).trim().to_string();
    }

    (value, field.value_mode)
}

fn should_transform_field(field: &str, fields: Option<&[String]>) -> bool {
    fields.is_some_and(|fields| {
        fields
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(field))
    })
}

fn abbreviate_month(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "jan" | "january" => Some("jan"),
        "2" | "feb" | "february" => Some("feb"),
        "3" | "mar" | "march" => Some("mar"),
        "4" | "apr" | "april" => Some("apr"),
        "5" | "may" => Some("may"),
        "6" | "jun" | "june" => Some("jun"),
        "7" | "jul" | "july" => Some("jul"),
        "8" | "aug" | "august" => Some("aug"),
        "9" | "sep" | "september" => Some("sep"),
        "10" | "oct" | "october" => Some("oct"),
        "11" | "nov" | "november" => Some("nov"),
        "12" | "dec" | "december" => Some("dec"),
        _ => None,
    }
}

fn encode_url(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'_') {
            chars.next();
            output.push_str("\\%5F");
        } else if ch == '_' {
            output.push_str("\\%5F");
        } else {
            output.push(ch);
        }
    }
    output
}

fn unwrap_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    let mut newline_count = 0usize;
    for ch in value.chars() {
        if ch == '\n' {
            newline_count += 1;
            pending_space = false;
            continue;
        }
        if ch.is_whitespace() {
            if newline_count >= 2 {
                continue;
            }
            pending_space = true;
            continue;
        }
        if newline_count >= 2 {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push_str("\n\n");
        } else if (newline_count == 1 || pending_space) && !output.is_empty() {
            output.push(' ');
        }
        newline_count = 0;
        pending_space = false;
        output.push(ch);
    }
    output
}

fn wrap_braced_value(field: &RawSyntaxField, value: &str, options: &TidyOptions) -> String {
    let indent = if options.tab {
        "\t".to_string()
    } else {
        " ".repeat(options.space)
    };
    let field_name = render_field_name(field, options);
    let inline_len =
        indent.len() + field_name.len() + separator(&field_name, options).len() + value.len() + 2;
    let multi_paragraph = value.contains("\n\n");
    let should_wrap = options.wrap.is_some_and(|wrap| inline_len > wrap) || multi_paragraph;
    if !should_wrap {
        return value.to_string();
    }

    let value_indent = indent.repeat(2);
    let paragraphs = value
        .split("\n\n")
        .map(|paragraph| {
            if let Some(wrap) = options.wrap {
                wrap_text(paragraph, wrap.saturating_sub(value_indent.len()))
                    .join(&format!("\n{value_indent}"))
            } else {
                paragraph.to_string()
            }
        })
        .collect::<Vec<_>>();
    format!(
        "\n{value_indent}{}\n{indent}",
        paragraphs.join(&format!("\n\n{value_indent}"))
    )
}

fn wrap_text(line: &str, line_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for (index, word) in line.split(' ').enumerate() {
        if current.len() + word.len() + 1 > line_width && index > 0 {
            lines.push(current.trim().to_string());
            current.clear();
        }
        current.push_str(word);
        current.push(' ');
    }
    lines.push(current.trim().to_string());
    lines
}

fn escape_characters(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '$'
            && let Some(end) = chars[index + 1..].iter().position(|ch| *ch == '$')
            && end > 0
        {
            let end = index + end + 1;
            output.extend(chars[index..=end].iter());
            index = end + 1;
            continue;
        }

        if chars[index] == '\\' {
            output.push(chars[index]);
            index += 1;
            if let Some(next) = chars.get(index) {
                output.push(*next);
                index += 1;
            }
            continue;
        }

        if let Some(escaped) = latex_escape(chars[index]) {
            output.push_str(escaped);
        } else {
            output.push(chars[index]);
        }
        index += 1;
    }
    output
}

fn is_verbatim_field(field: &str) -> bool {
    matches!(
        field.to_ascii_lowercase().as_str(),
        "url" | "doi" | "eprint" | "file" | "verba" | "verbb" | "verbc" | "pdf"
    )
}

fn drop_all_caps(value: &str) -> String {
    if value.chars().any(|ch| ch.is_lowercase()) {
        return value.to_string();
    }

    let mut output = String::with_capacity(value.len());
    let mut word = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            push_title_word(&mut output, &word);
            word.clear();
            output.push(ch);
        }
    }
    push_title_word(&mut output, &word);
    output
}

fn push_title_word(output: &mut String, word: &str) {
    if word.is_empty() {
        return;
    }
    if is_roman_numeral(word) {
        output.push_str(word);
        return;
    }

    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        output.extend(first.to_uppercase());
        for ch in chars {
            output.extend(ch.to_lowercase());
        }
    }
}

fn is_roman_numeral(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let chars = word.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    roman_take(&chars, &mut index, 'M', 0, 4);
    roman_take_digit(&chars, &mut index, 'C', 'D', 'M');
    roman_take_digit(&chars, &mut index, 'X', 'L', 'C');
    roman_take_digit(&chars, &mut index, 'I', 'V', 'X');
    index == chars.len()
}

fn roman_take(chars: &[char], index: &mut usize, ch: char, min: usize, max: usize) -> usize {
    let start = *index;
    while *index < chars.len() && chars[*index] == ch && *index - start < max {
        *index += 1;
    }
    let count = *index - start;
    if count < min {
        *index = start;
        0
    } else {
        count
    }
}

fn roman_take_pair(chars: &[char], index: &mut usize, first: char, second: char) -> bool {
    if chars.get(*index) == Some(&first) && chars.get(*index + 1) == Some(&second) {
        *index += 2;
        true
    } else {
        false
    }
}

fn roman_take_digit(chars: &[char], index: &mut usize, one: char, five: char, ten: char) {
    if roman_take_pair(chars, index, one, ten) || roman_take_pair(chars, index, one, five) {
        return;
    }
    roman_take(chars, index, five, 0, 1);
    roman_take(chars, index, one, 0, 3);
}

fn limit_authors(value: &str, max_authors: usize) -> String {
    let authors = value.split(" and ").collect::<Vec<_>>();
    if authors.len() > max_authors {
        authors
            .into_iter()
            .take(max_authors)
            .chain(std::iter::once("others"))
            .collect::<Vec<_>>()
            .join(" and ")
    } else {
        value.to_string()
    }
}

fn is_numeric_value(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('1'..='9')) && chars.all(|ch| ch.is_ascii_digit())
}

fn is_bare_month(field: &RawSyntaxField, value: &str) -> bool {
    field.name.eq_ignore_ascii_case("month")
        && matches!(
            value.to_ascii_lowercase().as_str(),
            "jan"
                | "feb"
                | "mar"
                | "apr"
                | "may"
                | "jun"
                | "jul"
                | "aug"
                | "sep"
                | "oct"
                | "nov"
                | "dec"
        )
        && field.value_mode == RawValueMode::Bare
}

fn format_page_range(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 1);
    for index in 0..chars.len() {
        let ch = chars[index];
        if ch == '-'
            && chars
                .get(index.wrapping_sub(1))
                .is_some_and(|left| left.is_ascii_digit())
            && chars
                .get(index + 1)
                .is_some_and(|right| right.is_ascii_digit())
            && chars.get(index + 1) != Some(&'-')
            && chars.get(index.wrapping_sub(1)) != Some(&'-')
        {
            output.push_str("--");
        } else {
            output.push(ch);
        }
    }
    output
}
