mod engine;
mod json;
mod source;

pub(crate) use json::{decode_records, parse_json};

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextKind {
    Normal,
    Protected,
    Math,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextChunk {
    pub kind: TextKind,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub chunks: Vec<TextChunk>,
    pub short: Option<Vec<TextChunk>>,
}

impl Text {
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            chunks: vec![TextChunk {
                kind: TextKind::Normal,
                text: value.into(),
            }],
            short: None,
        }
    }

    pub fn plain_text(&self) -> String {
        self.chunks
            .iter()
            .map(|chunk| match chunk.kind {
                TextKind::Math => format!("${}$", chunk.text),
                _ => chunk.text.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Name {
    Person {
        family: String,
        given: Option<String>,
        prefix: Option<String>,
        suffix: Option<String>,
        alias: Option<String>,
        #[serde(default)]
        comma_suffix: bool,
        id: Option<String>,
        given_initials: Option<String>,
        prefix_initials: Option<String>,
        use_prefix: Option<bool>,
        non_dropping_particle: Option<String>,
    },
    Organization {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contributors {
    pub role: String,
    pub names: Vec<Name>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DateParts {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub season: Option<u8>,
    pub time: Option<String>,
}

impl DateParts {
    fn year_text(&self) -> String {
        if self.year < 0 {
            format!("{:05}", self.year)
        } else {
            format!("{:04}", self.year)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DateValue {
    Point {
        date: DateParts,
    },
    Range {
        start: Option<DateParts>,
        end: Option<DateParts>,
    },
    Literal {
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Date {
    pub value: DateValue,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default)]
    pub approximate: bool,
}

impl Date {
    pub fn year(year: i32) -> Self {
        Self {
            value: DateValue::Point {
                date: DateParts {
                    year,
                    month: None,
                    day: None,
                    season: None,
                    time: None,
                },
            },
            uncertain: false,
            approximate: false,
        }
    }

    pub fn display(&self) -> String {
        fn point(date: &DateParts) -> String {
            let mut result = date.year_text();
            if let Some(month) = date.month {
                result.push_str(&format!("-{month:02}"));
            }
            if let Some(day) = date.day {
                result.push_str(&format!("-{day:02}"));
            }
            if let Some(season) = date.season {
                result.push_str(&format!("-S{season}"));
            }
            if let Some(time) = &date.time {
                result.push('T');
                result.push_str(time);
            }
            result
        }
        let mut result = match &self.value {
            DateValue::Point { date } => point(date),
            DateValue::Range { start, end } => format!(
                "{}/{}",
                start.as_ref().map(point).unwrap_or_default(),
                end.as_ref().map(point).unwrap_or_default()
            ),
            DateValue::Literal { text } => text.clone(),
        };
        match (self.uncertain, self.approximate) {
            (true, true) => result.push('%'),
            (true, false) => result.push('?'),
            (false, true) => result.push('~'),
            _ => {}
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ScalarValue {
    Typed(String),
    Literal(String),
}

impl ScalarValue {
    pub fn value(&self) -> &str {
        match self {
            Self::Typed(value) | Self::Literal(value) => value,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Publisher {
    pub name: Option<Text>,
    pub location: Option<Text>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Url {
    pub value: String,
    pub accessed: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ExtensionValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<ExtensionValue>),
    Object(BTreeMap<String, ExtensionValue>),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EntryRecord {
    pub key: String,
    pub entry_type: String,
    pub title: Option<Text>,
    pub authors: Vec<Name>,
    pub editors: Vec<Name>,
    pub affiliated: Vec<Contributors>,
    pub date: Option<Date>,
    pub event_date: Option<Date>,
    pub original_date: Option<Date>,
    pub publisher: Option<Publisher>,
    pub location: Option<Text>,
    pub organization: Option<Text>,
    pub issue: Option<ScalarValue>,
    pub chapter: Option<ScalarValue>,
    pub volume: Option<ScalarValue>,
    pub volume_total: Option<ScalarValue>,
    pub edition: Option<ScalarValue>,
    pub page_range: Option<ScalarValue>,
    pub page_total: Option<ScalarValue>,
    pub time_range: Option<ScalarValue>,
    pub runtime: Option<ScalarValue>,
    pub url: Option<Url>,
    pub identifiers: BTreeMap<String, String>,
    pub language: Option<String>,
    pub archive: Option<Text>,
    pub archive_location: Option<Text>,
    pub call_number: Option<Text>,
    pub note: Option<Text>,
    pub abstract_text: Option<Text>,
    pub genre: Option<Text>,
    pub keywords: Vec<String>,
    pub parents: Vec<EntryRecord>,
    pub extensions: BTreeMap<String, BTreeMap<String, ExtensionValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordError {
    pub path: String,
    pub message: String,
}

impl RecordError {
    pub(crate) fn new(path: impl Into<String>, message: impl ToString) -> Self {
        Self {
            path: path.into(),
            message: message.to_string(),
        }
    }
}

impl fmt::Display for RecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for RecordError {}

pub(crate) const MAX_RECORD_JSON_BYTES: usize = 128 * 1024 * 1024;

pub fn validate_record_source(source: &str) -> Result<(), RecordError> {
    if source.len() > MAX_RECORD_JSON_BYTES {
        return Err(RecordError::new(
            "records",
            "record serialization exceeds 128 MiB",
        ));
    }
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for byte in source.bytes() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else {
            match byte {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 256 {
                        return Err(RecordError::new(
                            "records",
                            "record JSON nesting exceeds 256 levels",
                        ));
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    Ok(())
}
