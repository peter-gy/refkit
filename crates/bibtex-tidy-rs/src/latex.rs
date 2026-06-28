pub(crate) fn double_enclose(value: &str) -> String {
    if has_single_outer_brace(value) {
        value.to_string()
    } else {
        format!("{{{value}}}")
    }
}

pub(crate) fn strip_simple_outer_brace(value: &str) -> String {
    if has_single_outer_brace(value) {
        let inner = &value[1..value.len() - 1];
        if !inner.contains(['{', '}']) {
            return inner.to_string();
        }
    }
    value.to_string()
}

pub(crate) fn flatten_unprotected_braces(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    flatten_until(&chars, &mut index, None, false)
}

fn flatten_until(
    chars: &[char],
    index: &mut usize,
    closing: Option<char>,
    preserve_current_block: bool,
) -> String {
    let mut output = String::new();
    while *index < chars.len() {
        let ch = chars[*index];
        if Some(ch) == closing {
            *index += 1;
            break;
        }
        match ch {
            '\\' => output.push_str(&read_command(chars, index)),
            '{' => {
                *index += 1;
                let inner = flatten_until(chars, index, Some('}'), false);
                if preserve_current_block {
                    output.push('{');
                    output.push_str(&inner);
                    output.push('}');
                } else {
                    output.push_str(&inner);
                }
            }
            _ => {
                output.push(ch);
                *index += 1;
            }
        }
    }
    output
}

fn read_command(chars: &[char], index: &mut usize) -> String {
    let mut output = String::from("\\");
    *index += 1;
    while *index < chars.len() {
        let ch = chars[*index];
        if ch == '{' || ch == '[' || ch == '}' || ch == ']' || ch.is_whitespace() {
            break;
        }
        output.push(ch);
        *index += 1;
    }
    loop {
        if *index >= chars.len() {
            break;
        }
        match chars[*index] {
            '{' => {
                *index += 1;
                output.push('{');
                output.push_str(&flatten_until(chars, index, Some('}'), true));
                output.push('}');
            }
            '[' => {
                *index += 1;
                output.push('[');
                output.push_str(&flatten_until(chars, index, Some(']'), true));
                output.push(']');
            }
            _ => break,
        }
    }
    output
}

fn has_single_outer_brace(value: &str) -> bool {
    if !value.starts_with('{') || !value.ends_with('}') || value.len() < 2 {
        return false;
    }

    let mut depth = 0usize;
    for (index, ch) in value.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
                if depth == 0 && index + ch.len_utf8() < value.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_unprotected_braces_preserves_command_arguments() {
        assert_eq!(
            flatten_unprotected_braces(r"Quantifying \textbf{Madness} in {Green {Leaf}} Ants"),
            r"Quantifying \textbf{Madness} in Green Leaf Ants",
        );
    }

    #[test]
    fn double_enclose_keeps_existing_outer_value_brace() {
        assert_eq!(
            double_enclose("{Quantum somethings}"),
            "{Quantum somethings}"
        );
        assert_eq!(double_enclose("Quantum somethings"), "{Quantum somethings}");
    }
}
