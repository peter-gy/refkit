pub(super) fn is_orcid(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    ["https://orcid.org/", "http://orcid.org/", "orcid:"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
}

pub(crate) fn canonical(kind: &str, value: &str) -> Option<Result<String, ()>> {
    let kind = kind.to_ascii_lowercase();
    let value = value.trim();
    Some(match kind.as_str() {
        "doi" => doi(value),
        "isbn" => isbn(value),
        "issn" => issn(value),
        "orcid" => orcid(value),
        _ => return None,
    })
}

fn strip_prefix<'a>(value: &'a str, prefixes: &[&str]) -> &'a str {
    for prefix in prefixes {
        if value
            .get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
        {
            return &value[prefix.len()..];
        }
    }
    value
}

fn doi(value: &str) -> Result<String, ()> {
    let value = strip_prefix(
        value,
        &[
            "https://doi.org/",
            "http://doi.org/",
            "https://dx.doi.org/",
            "http://dx.doi.org/",
            "doi:",
        ],
    )
    .trim();
    let (prefix, suffix) = value.split_once('/').ok_or(())?;
    let registrant = prefix.strip_prefix("10.").ok_or(())?;
    if registrant
        .split('.')
        .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
        || suffix.is_empty()
        || value.chars().any(char::is_control)
    {
        return Err(());
    }
    Ok(value.to_ascii_lowercase())
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|c| *c != '-' && *c != ' ')
        .collect::<String>()
        .to_ascii_uppercase()
}

fn digits(value: &str, length: usize) -> Result<Vec<u32>, ()> {
    if value.len() != length {
        return Err(());
    }
    value
        .bytes()
        .enumerate()
        .map(|(i, byte)| match byte {
            b'0'..=b'9' => Ok(u32::from(byte - b'0')),
            b'X' if i + 1 == length => Ok(10),
            _ => Err(()),
        })
        .collect()
}

fn isbn(value: &str) -> Result<String, ()> {
    let value = compact(value);
    match value.len() {
        10 => {
            let digits = digits(&value, 10)?;
            if digits
                .iter()
                .zip((1..=10).rev())
                .map(|(d, w)| d * w)
                .sum::<u32>()
                % 11
                != 0
            {
                return Err(());
            }
        }
        13 => {
            let digits = digits(&value, 13)?;
            if !(value.starts_with("978") || value.starts_with("979"))
                || digits[12] == 10
                || digits
                    .iter()
                    .enumerate()
                    .map(|(i, d)| d * if i % 2 == 0 { 1 } else { 3 })
                    .sum::<u32>()
                    % 10
                    != 0
            {
                return Err(());
            }
        }
        _ => return Err(()),
    }
    Ok(value)
}

fn issn(value: &str) -> Result<String, ()> {
    let value = compact(value);
    let digits = digits(&value, 8)?;
    if digits
        .iter()
        .zip((1..=8).rev())
        .map(|(d, w)| d * w)
        .sum::<u32>()
        % 11
        != 0
    {
        return Err(());
    }
    Ok(format!("{}-{}", &value[..4], &value[4..]))
}

fn orcid(value: &str) -> Result<String, ()> {
    let value = compact(strip_prefix(
        value,
        &["https://orcid.org/", "http://orcid.org/", "orcid:"],
    ));
    let digits = digits(&value, 16)?;
    let total = digits[..15]
        .iter()
        .fold(0, |total, digit| (total + digit) * 2);
    if (12 - total % 11) % 11 != digits[15] {
        return Err(());
    }
    Ok(format!(
        "https://orcid.org/{}-{}-{}-{}",
        &value[..4],
        &value[4..8],
        &value[8..12],
        &value[12..]
    ))
}
