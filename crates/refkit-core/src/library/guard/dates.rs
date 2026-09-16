pub(super) fn is_plain_date_text(value: &str) -> bool {
    // These ASCII characters are unchanged by BibLaTeX text normalization.
    value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(byte, b' ' | b'+' | b'-' | b'/' | b'?' | b'.' | b':')
    }) && !value.contains("--")
}

fn date_part(name: &str) -> &str {
    ["url", "orig", "event"]
        .iter()
        .find_map(|prefix| name.strip_prefix(*prefix))
        .unwrap_or(name)
}

pub(super) fn is_date_parser_field(name: &str) -> bool {
    matches!(date_part(name), "date" | "year" | "month")
}

pub(crate) fn validate_date_parser_input(name: &str, value: &str) -> Result<(), String> {
    let name = name.to_ascii_lowercase();
    let value = value.trim_start();
    match date_part(&name) {
        "date" => {
            validate_uncertain_date(value)?;
            validate_date_numbers(value)
        }
        "year" => {
            if value.strip_prefix(['+', '-']).is_some_and(|unsigned| {
                let digits = unsigned
                    .trim_start()
                    .bytes()
                    .take_while(u8::is_ascii_digit)
                    .count();
                unsigned.starts_with(char::is_whitespace) && (1..=4).contains(&digits)
            }) {
                return Err("BibLaTeX year signs must directly precede their digits".into());
            }
            Ok(())
        }
        "month" => validate_month(value),
        _ => Ok(()),
    }
}

fn validate_uncertain_date(value: &str) -> Result<(), String> {
    if !value.contains(['X', 'x']) {
        return Ok(());
    }
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 4 {
        return Err("BibLaTeX uncertain-year dates require a four-digit year pattern".into());
    }
    if digits == 4 {
        let suffix = value.get(digits..).unwrap_or_default().trim_start();
        let month = suffix.trim_start_matches('-').trim_start();
        if suffix.starts_with('-') && month.starts_with("00") {
            return Err("BibLaTeX numeric months start at one".into());
        }
    }
    Ok(())
}

fn validate_month(value: &str) -> Result<(), String> {
    let mut cursor = value;
    match numeric_token(&mut cursor, &[1, 2])? {
        Some(0) => return Err("BibLaTeX numeric months start at one".into()),
        Some(_) => {}
        None => {
            cursor = cursor.trim_start_matches(|character: char| character.is_ascii_alphabetic());
        }
    }
    let tail =
        cursor.trim_start_matches(|character: char| character.is_whitespace() || character == '-');
    if tail.len() == cursor.len() {
        return Ok(());
    }
    let day_len = tail.bytes().take_while(u8::is_ascii_digit).count();
    if day_len > 0
        && tail
            .get(..day_len)
            .is_none_or(|day| day.parse::<u8>().is_err())
    {
        return Err("BibLaTeX day embedded in a month field exceeds the numeric range".into());
    }
    Ok(())
}

fn numeric_token(cursor: &mut &str, widths: &[usize]) -> Result<Option<u16>, String> {
    // The pinned parser tests byte width before unwrapping ASCII integer parsing.
    *cursor = cursor.trim_start();
    let suffix = cursor.trim_start_matches(char::is_numeric);
    let width = cursor.len() - suffix.len();
    let digits = cursor.get(..width).unwrap_or_default();
    *cursor = suffix;
    if !widths.contains(&width) {
        return Ok(None);
    }
    digits
        .parse()
        .map(Some)
        .map_err(|_| "BibLaTeX date numbers require ASCII digits".into())
}

fn validate_date_numbers(value: &str) -> Result<(), String> {
    if value.contains(['X', 'x'])
        || !value
            .chars()
            .any(|character| character.is_numeric() && !character.is_ascii_digit())
    {
        return Ok(());
    }
    let (first, second) = value
        .split_once('/')
        .map_or((value, None), |(first, second)| (first, Some(second)));
    validate_datetime_numbers(first)?;
    if let Some(second) = second
        && (first.trim().is_empty()
            || first == ".."
            || (!first.trim_end().ends_with(['?', '~', '%']) && parsed_date(first)))
    {
        validate_datetime_numbers(second)?;
    }
    Ok(())
}

fn parsed_date(value: &str) -> bool {
    biblatex::Date::parse(&[biblatex::Spanned::detached(biblatex::Chunk::Normal(
        value.to_string(),
    ))])
    .is_ok()
}

fn validate_datetime_numbers(value: &str) -> Result<(), String> {
    // Stop where upstream stops, preserving arbitrary literal date text after that point.
    let mut cursor = value.trim_start();
    cursor = cursor.strip_prefix(['+', '-']).unwrap_or(cursor);
    if numeric_token(&mut cursor, &[2, 4])?.is_none() {
        return Ok(());
    }
    for (separator, minimum, maximum) in [
        ('-', 1, 12),
        ('-', 1, 31),
        ('T', 0, 23),
        (':', 0, 59),
        (':', 0, 59),
    ] {
        cursor = cursor.trim_start();
        let suffix = match separator {
            'T' => cursor.strip_prefix(['T', 't']),
            _ => cursor.strip_prefix(separator),
        };
        let Some(suffix) = suffix else {
            return Ok(());
        };
        cursor = if separator == '-' {
            suffix.trim_start_matches('-')
        } else {
            suffix
        };
        let Some(number) = numeric_token(&mut cursor, &[1, 2])? else {
            return Ok(());
        };
        if !(minimum..=maximum).contains(&number) {
            return Ok(());
        }
        if separator == '-'
            && maximum == 31
            && !parsed_date(value.get(..value.len() - cursor.len()).unwrap_or_default())
        {
            return Ok(());
        }
    }
    cursor = cursor.trim_start();
    let Some(suffix) = cursor.strip_prefix(['+', '-']) else {
        return Ok(());
    };
    cursor = suffix;
    if numeric_token(&mut cursor, &[1, 2])?.is_none_or(|number| number > 23) {
        return Ok(());
    }
    if let Some(suffix) = cursor.trim_start().strip_prefix(':') {
        cursor = suffix;
        let _ = numeric_token(&mut cursor, &[1, 2])?;
    }
    Ok(())
}
