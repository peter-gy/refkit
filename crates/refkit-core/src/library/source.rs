use std::ops::Range;

pub(super) fn mask(source: &mut String, span: Range<usize>) {
    let replacement = source[span.clone()]
        .bytes()
        .map(|byte| {
            if matches!(byte, b'\n' | b'\r') {
                char::from(byte)
            } else {
                ' '
            }
        })
        .collect::<String>();
    source.replace_range(span, &replacement);
}
