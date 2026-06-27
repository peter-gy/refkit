use std::fs;
use std::path::Path;

pub struct SourceText {
    pub source: String,
    pub diagnostic: Option<String>,
}

pub fn read_bibliography_text(path: &Path) -> Result<SourceText, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    match String::from_utf8(bytes) {
        Ok(source) => Ok(SourceText {
            source,
            diagnostic: None,
        }),
        Err(err) => Ok(SourceText {
            source: decode_windows_1252(&err.into_bytes()),
            diagnostic: Some(format!(
                "decoded {} as Windows-1252-compatible text because it is not valid UTF-8",
                path.display()
            )),
        }),
    }
}

fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match byte {
            0x80 => '\u{20ac}',
            0x82 => '\u{201a}',
            0x83 => '\u{0192}',
            0x84 => '\u{201e}',
            0x85 => '\u{2026}',
            0x86 => '\u{2020}',
            0x87 => '\u{2021}',
            0x88 => '\u{02c6}',
            0x89 => '\u{2030}',
            0x8a => '\u{0160}',
            0x8b => '\u{2039}',
            0x8c => '\u{0152}',
            0x8e => '\u{017d}',
            0x91 => '\u{2018}',
            0x92 => '\u{2019}',
            0x93 => '\u{201c}',
            0x94 => '\u{201d}',
            0x95 => '\u{2022}',
            0x96 => '\u{2013}',
            0x97 => '\u{2014}',
            0x98 => '\u{02dc}',
            0x99 => '\u{2122}',
            0x9a => '\u{0161}',
            0x9b => '\u{203a}',
            0x9c => '\u{0153}',
            0x9e => '\u{017e}',
            0x9f => '\u{0178}',
            0x81 | 0x8d | 0x8f | 0x90 | 0x9d => '\u{fffd}',
            _ => char::from(*byte),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_windows_1252_when_utf8_fails() {
        assert_eq!(decode_windows_1252(&[0x48, 0x80]), "H€");
    }
}
