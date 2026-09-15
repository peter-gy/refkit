use crate::{DuplicateRule, raw::RawSyntaxEntry};

pub(crate) fn signature(entry: &RawSyntaxEntry, rule: DuplicateRule) -> Option<String> {
    let field = |name: &str| {
        entry
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(name))
            .map(|field| field.value.as_str())
    };
    let result = match rule {
        DuplicateRule::Key => entry.key.to_ascii_lowercase(),
        DuplicateRule::Doi => alpha_num(field("doi")?),
        DuplicateRule::Abstract => alpha_num(field("abstract")?).chars().take(100).collect(),
        DuplicateRule::Citation => [
            alpha_num(&first_author_last(field("author")?)),
            alpha_num(field("title")?),
            alpha_num(field("number").unwrap_or("0")),
        ]
        .join(":"),
    };
    (!result.is_empty()).then_some(result)
}

fn first_author_last(author: &str) -> String {
    let author = author.split(" and ").next().unwrap_or(author).trim();
    if let Some((last, _)) = author.split_once(',') {
        return last.trim().into();
    }
    let parts = author.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [] => String::new(),
        [last] => (*last).into(),
        [_, last @ ..] => last.join(" "),
    }
}

fn alpha_num(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
