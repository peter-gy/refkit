use std::collections::HashMap;

use crate::{
    RawEntryId, RawSyntaxDocument, RawSyntaxEntry, RawSyntaxField, RawValueAtom, RawValueMode,
};

use super::TidyOptions;

const MISSING_REQUIRED_DATA: &str = "missing required citation key data";

#[derive(Debug, Clone, PartialEq, Eq)]
enum TemplateToken {
    Text(String),
    Marker {
        marker: String,
        parameter: Option<usize>,
        modifiers: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Name {
    last: String,
}

pub(crate) fn generated_keys(
    doc: &RawSyntaxDocument,
    options: &TidyOptions,
) -> Result<HashMap<RawEntryId, String>, String> {
    let Some(template) = options.generate_keys.as_deref() else {
        return Ok(HashMap::new());
    };
    let template =
        if template.contains("[duplicateLetter]") || template.contains("[duplicateNumber]") {
            template.to_string()
        } else {
            format!("{template}[duplicateLetter]")
        };
    let template = parse_template(&template)?;
    let mut entries_by_key: Vec<(String, Vec<&RawSyntaxEntry>)> = Vec::new();

    for entry in &doc.entries {
        let values = entry_values(entry);
        if let Some(key) = generate_key(&values, &template, None)? {
            if let Some((_, entries)) = entries_by_key
                .iter_mut()
                .find(|(candidate, _)| candidate == &key)
            {
                entries.push(entry);
            } else {
                entries_by_key.push((key, vec![entry]));
            }
        }
    }

    let mut generated = HashMap::new();
    for (key, entries) in entries_by_key {
        let regenerate_duplicate = entries.len() > 1;
        for (index, entry) in entries.into_iter().enumerate() {
            let new_key = if regenerate_duplicate {
                let values = entry_values(entry);
                generate_key(&values, &template, Some(index + 1))?
            } else {
                Some(key.clone())
            };
            if let Some(new_key) = new_key {
                generated.insert(entry.id, new_key);
            }
        }
    }

    Ok(generated)
}

fn parse_template(template: &str) -> Result<Vec<TemplateToken>, String> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    while cursor < template.len() {
        let Some(open_rel) = template[cursor..].find('[') else {
            tokens.push(TemplateToken::Text(template[cursor..].to_string()));
            break;
        };
        let open = cursor + open_rel;
        if open > cursor {
            tokens.push(TemplateToken::Text(template[cursor..open].to_string()));
        }
        let Some(close_rel) = template[open..].find(']') else {
            tokens.push(TemplateToken::Text(template[open..].to_string()));
            break;
        };
        let close = open + close_rel;
        let parts = template[open + 1..close].split(':').collect::<Vec<_>>();
        let Some(marker_with_parameter) = parts.first().copied().filter(|part| !part.is_empty())
        else {
            return Err("Token parse error".to_string());
        };
        let (marker, parameter) = marker_parameter(marker_with_parameter);
        tokens.push(TemplateToken::Marker {
            marker,
            parameter,
            modifiers: parts[1..].iter().map(|part| (*part).to_string()).collect(),
        });
        cursor = close + 1;
    }
    Ok(tokens)
}

fn marker_parameter(marker: &str) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut parameter = None;
    let mut chars = marker.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() {
            let mut number = ch.to_string();
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    number.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            parameter = number.parse::<usize>().ok();
            out.push('N');
        } else {
            out.push(ch);
        }
    }
    (out, parameter)
}

fn generate_key(
    values: &HashMap<String, String>,
    template: &[TemplateToken],
    duplicate: Option<usize>,
) -> Result<Option<String>, String> {
    let mut key = String::new();
    for token in template {
        match token {
            TemplateToken::Text(value) => key.push_str(value),
            TemplateToken::Marker {
                marker,
                parameter,
                modifiers,
            } => {
                let mut words = marker_words(values, marker, *parameter, duplicate)?;
                for modifier in modifiers {
                    words = match apply_modifier(words, modifier) {
                        Ok(words) => words,
                        Err(error) if error == MISSING_REQUIRED_DATA => return Ok(None),
                        Err(error) => return Err(error),
                    };
                }
                key.push_str(&words.join(""));
            }
        }
    }

    let key = remove_unsafe_key_chars(&key);
    if key.is_empty() {
        Ok(None)
    } else {
        Ok(Some(key))
    }
}

fn marker_words(
    values: &HashMap<String, String>,
    marker: &str,
    parameter: Option<usize>,
    duplicate: Option<usize>,
) -> Result<Vec<String>, String> {
    match marker {
        "auth" => Ok(parse_name_list(value(values, "author"))
            .into_iter()
            .filter(|name| !name.last.is_empty())
            .collect::<Vec<_>>()
            .first()
            .map(|name| vec![name.last.clone()])
            .unwrap_or_default()),
        "authEtAl" => {
            let authors = parse_name_list(value(values, "author"))
                .into_iter()
                .filter(|name| !name.last.is_empty())
                .collect::<Vec<_>>();
            let mut words = authors
                .iter()
                .take(2)
                .map(|name| name.last.clone())
                .collect::<Vec<_>>();
            if authors.len() > 2 {
                words.extend(["Et".to_string(), "Al".to_string()]);
            }
            Ok(words)
        }
        "authors" => Ok(parse_name_list(value(values, "author"))
            .into_iter()
            .filter(|name| !name.last.is_empty())
            .map(|name| name.last)
            .collect()),
        "authorsN" => {
            let limit = parameter.unwrap_or(0);
            let authors = parse_name_list(value(values, "author"))
                .into_iter()
                .filter(|name| !name.last.is_empty())
                .collect::<Vec<_>>();
            let mut words = authors
                .iter()
                .take(limit)
                .map(|name| name.last.clone())
                .collect::<Vec<_>>();
            if authors.len() > limit {
                words.extend(["Et".to_string(), "Al".to_string()]);
            }
            Ok(words)
        }
        "veryshorttitle" => Ok(non_function_words(&title(values))
            .into_iter()
            .take(1)
            .collect()),
        "shorttitle" => Ok(non_function_words(&title(values))
            .into_iter()
            .take(3)
            .collect()),
        "title" => Ok(capitalize(words(&title(values)))),
        "fulltitle" => Ok(words(&title(values))),
        "year" => {
            let year = value(values, "year")
                .chars()
                .filter(char::is_ascii_digit)
                .collect::<String>();
            Ok(if year.is_empty() {
                Vec::new()
            } else {
                vec![year]
            })
        }
        "duplicateLetter" => Ok(duplicate
            .map(|value| vec![num_to_letter(value).to_string()])
            .unwrap_or_default()),
        "duplicateNumber" => Ok(vec![
            duplicate.map(|value| value.to_string()).unwrap_or_default(),
        ]),
        marker if marker == marker.to_uppercase() => {
            Ok(words(value(values, &marker.to_lowercase())))
        }
        _ => Err(format!("Invalid citation key token {marker}")),
    }
}

fn apply_modifier(words: Vec<String>, modifier: &str) -> Result<Vec<String>, String> {
    match modifier {
        "required" => {
            if words.is_empty() {
                Err(MISSING_REQUIRED_DATA.to_string())
            } else {
                Ok(words)
            }
        }
        "lower" => Ok(words.into_iter().map(|word| word.to_lowercase()).collect()),
        "upper" => Ok(words.into_iter().map(|word| word.to_uppercase()).collect()),
        "capitalize" => Ok(capitalize(words)),
        _ => Err(format!("Invalid modifier {modifier}")),
    }
}

fn entry_values(entry: &RawSyntaxEntry) -> HashMap<String, String> {
    entry
        .fields
        .iter()
        .map(|field| (field.name.to_lowercase(), rendered_field_value(field)))
        .collect()
}

fn rendered_field_value(field: &RawSyntaxField) -> String {
    match field.value_mode {
        RawValueMode::Expression => expression_text(&field.value_atoms),
        RawValueMode::Bare
        | RawValueMode::Braced
        | RawValueMode::Missing
        | RawValueMode::Quoted => field.value.clone(),
    }
}

fn expression_text(atoms: &[RawValueAtom]) -> String {
    atoms
        .iter()
        .map(|atom| atom.value.clone())
        .filter(|atom| !atom.is_empty())
        .collect::<Vec<_>>()
        .join(" # ")
}

fn parse_name_list(value: &str) -> Vec<Name> {
    value
        .split(|_| false)
        .next()
        .unwrap_or(value)
        .split(" and ")
        .flat_map(|part| split_uppercase_and(part).into_iter())
        .map(parse_name)
        .collect()
}

fn split_uppercase_and(value: &str) -> Vec<&str> {
    value.split(" AND ").collect()
}

fn parse_name(name: &str) -> Name {
    let tokens = tokenize_name(name.trim());
    let commas = tokens.iter().filter(|token| token.as_str() == ",").count();
    let last = if tokens.is_empty() {
        String::new()
    } else if tokens.len() == 1 && tokens[0] == "others" {
        "others".to_string()
    } else if commas > 0 {
        tokens
            .iter()
            .take_while(|token| token.as_str() != ",")
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else if let Some(prefix_index) = tokens.iter().position(|token| is_prefix_token(token)) {
        tokens[prefix_index..]
            .iter()
            .skip_while(|token| is_prefix_token(token))
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    } else if tokens.len() == 1 {
        tokens[0].clone()
    } else {
        tokens[1..].join(" ")
    };
    Name { last }
}

fn tokenize_name(name: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in name.chars() {
        if ch == ',' {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(",".to_string());
        } else if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn is_prefix_token(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
}

fn title(values: &HashMap<String, String>) -> String {
    let title = value(values, "title");
    if title.is_empty() {
        value(values, "booktitle").to_string()
    } else {
        title.to_string()
    }
}

fn value<'a>(values: &'a HashMap<String, String>, key: &str) -> &'a str {
    values.get(key).map(String::as_str).unwrap_or("")
}

fn non_function_words(value: &str) -> Vec<String> {
    words(value)
        .into_iter()
        .filter(|word| !is_function_word(word))
        .collect()
}

fn words(value: &str) -> Vec<String> {
    value
        .split(|ch: char| ch.is_whitespace() || matches!(ch, '.' | ',' | ':' | ';'))
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn capitalize(words: Vec<String>) -> Vec<String> {
    words
        .into_iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            first
                .to_uppercase()
                .chain(chars.flat_map(char::to_lowercase))
                .collect()
        })
        .collect()
}

fn is_function_word(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "a" | "about"
            | "above"
            | "across"
            | "against"
            | "along"
            | "among"
            | "an"
            | "and"
            | "around"
            | "at"
            | "before"
            | "behind"
            | "below"
            | "beneath"
            | "beside"
            | "between"
            | "beyond"
            | "but"
            | "by"
            | "down"
            | "during"
            | "except"
            | "for"
            | "from"
            | "in"
            | "inside"
            | "into"
            | "like"
            | "near"
            | "nor"
            | "of"
            | "off"
            | "on"
            | "onto"
            | "or"
            | "since"
            | "so"
            | "the"
            | "through"
            | "to"
            | "toward"
            | "under"
            | "until"
            | "up"
            | "upon"
            | "with"
            | "within"
            | "without"
            | "yet"
    )
}

fn num_to_letter(value: usize) -> char {
    char::from_u32(96 + value as u32).unwrap_or_default()
}

fn remove_unsafe_key_chars(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                '{' | '}'
                    | ','
                    | '\\'
                    | '#'
                    | '%'
                    | '~'
                    | '('
                    | ')'
                    | '"'
                    | '\''
                    | '='
                    | '.'
                    | ':'
                    | ';'
                    | '['
                    | ']'
                    | '_'
            ) && !ch.is_whitespace()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generated(entry: &[(&str, &str)], template: &str) -> String {
        let values = entry
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<HashMap<_, _>>();
        generate_key(&values, &parse_template(template).unwrap(), None)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn generates_author_year_title_keys() {
        assert_eq!(
            generated(
                &[
                    ("author", "Bar, Foo and Mee, Moo"),
                    ("year", "2018"),
                    ("title", "A story of 2 foo and 1 bar: the best story")
                ],
                "[auth:upper][year][shorttitle:capitalize]"
            ),
            "BAR2018Story2Foo"
        );
    }

    #[test]
    fn keeps_existing_key_when_required_data_is_missing() {
        let values = HashMap::from([("title".to_string(), "No Author".to_string())]);
        let key = generate_key(
            &values,
            &parse_template("[auth:required][title]").unwrap(),
            None,
        )
        .unwrap();

        assert_eq!(key, None);
    }
}
