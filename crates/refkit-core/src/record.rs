mod engine;
mod json;
mod source;

pub(crate) use json::{decode_records, parse_json};

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Treatment of a text fragment during case conversion and rendering.
pub enum TextKind {
    /// Text whose capitalization a style may change.
    Normal,
    /// Text whose capitalization must be preserved.
    Protected,
    /// Mathematical content without its surrounding dollar delimiters.
    Math,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One contiguous fragment with a single rendering treatment.
pub struct TextChunk {
    /// Case protection or mathematical interpretation of the fragment.
    pub kind: TextKind,
    /// Unicode content of the fragment.
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Ordered rich text with an optional abbreviated form.
pub struct Text {
    /// Fragments of the full form, in reading order.
    pub chunks: Vec<TextChunk>,
    /// Explicit short form, distinct from an absent short form.
    pub short: Option<Vec<TextChunk>>,
}

impl Text {
    /// Construct one unprotected fragment with no short form.
    #[must_use]
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            chunks: vec![TextChunk {
                kind: TextKind::Normal,
                text: value.into(),
            }],
            short: None,
        }
    }

    #[must_use]
    /// Flatten the full form, enclosing mathematical fragments in dollar signs.
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
/// A structured personal name or a literal organizational creator.
pub enum Name {
    /// Components retained independently for style-dependent name ordering.
    Person {
        /// Family name without the separately stored particles.
        family: String,
        /// Given names in their supplied order.
        given: Option<String>,
        /// Name prefix supplied by the source format.
        prefix: Option<String>,
        /// Generational or other name suffix.
        suffix: Option<String>,
        /// Alternate name supplied by the source.
        alias: Option<String>,
        #[serde(default)]
        /// Whether a comma separates the suffix from the preceding name.
        comma_suffix: bool,
        /// Source-provided creator identifier.
        id: Option<String>,
        /// Explicit initials overriding automatic initialization of given names.
        given_initials: Option<String>,
        /// Explicit initials for the prefix.
        prefix_initials: Option<String>,
        /// Source instruction for including the prefix in name treatment.
        use_prefix: Option<bool>,
        /// CSL particle retained with the family name in shortened forms.
        non_dropping_particle: Option<String>,
    },
    /// An institution or group that must not be split into personal components.
    Organization {
        /// Complete literal organization name.
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Creators sharing a role other than the primary author or editor list.
pub struct Contributors {
    /// Role vocabulary understood by the source and supported rendering profile.
    pub role: String,
    /// Creators in source order.
    pub names: Vec<Name>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Calendar components whose presence records the supplied precision.
pub struct DateParts {
    /// Signed calendar year.
    pub year: i32,
    /// One-based calendar month, when known.
    pub month: Option<u8>,
    /// One-based day of the month, when known.
    pub day: Option<u8>,
    /// Seasonal date code, distinct from a calendar month.
    pub season: Option<u8>,
    /// Clock time with any supplied UTC marker or offset.
    pub time: Option<String>,
}

impl DateParts {
    #[expect(
        clippy::expect_used,
        reason = "Only standard integer and string formatting is written into String, whose fmt::Write implementation cannot fail."
    )]
    fn display(&self) -> String {
        let mut result = self.year_text();
        if let Some(month) = self.month {
            write!(result, "-{month:02}").expect("String formatting is infallible");
        }
        if let Some(day) = self.day {
            write!(result, "-{day:02}").expect("String formatting is infallible");
        }
        if let Some(season) = self.season {
            write!(result, "-S{season}").expect("String formatting is infallible");
        }
        if let Some(time) = &self.time {
            result.push('T');
            result.push_str(time);
        }
        result
    }

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
/// A calendar point, an interval, or uninterpreted date text.
pub enum DateValue {
    /// One date at the precision supplied by its components.
    Point {
        /// Calendar components of the point.
        date: DateParts,
    },
    /// An interval with optional open endpoints.
    Range {
        /// Lower endpoint, or no known lower bound.
        start: Option<DateParts>,
        /// Upper endpoint, or no known upper bound.
        end: Option<DateParts>,
    },
    /// Date text that is retained without calendar interpretation.
    Literal {
        /// Original literal date value.
        text: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A date with independent uncertainty and approximation qualifiers.
pub struct Date {
    /// Calendar or literal representation.
    pub value: DateValue,
    #[serde(default)]
    /// Whether the supplied date is uncertain.
    pub uncertain: bool,
    #[serde(default)]
    /// Whether the supplied date is approximate.
    pub approximate: bool,
}

impl Date {
    #[must_use]
    /// Construct an unqualified date with year precision.
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

    /// Format retained components, interval bounds, and qualification markers.
    #[must_use]
    pub fn display(&self) -> String {
        let mut result = match &self.value {
            DateValue::Point { date } => date.display(),
            DateValue::Range { start, end } => format!(
                "{}/{}",
                start.as_ref().map(DateParts::display).unwrap_or_default(),
                end.as_ref().map(DateParts::display).unwrap_or_default()
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
/// A field interpreted by its domain parser or retained as a literal.
pub enum ScalarValue {
    /// Text accepted by the field's numeric, range, or duration parser.
    Typed(String),
    /// Text retained without that typed interpretation.
    Literal(String),
}

impl ScalarValue {
    #[must_use]
    /// Borrow the stored representation without changing its interpretation.
    pub fn value(&self) -> &str {
        match self {
            Self::Typed(value) | Self::Literal(value) => value,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// Publisher identity and place of publication.
pub struct Publisher {
    /// Publisher's formatted name.
    pub name: Option<Text>,
    /// Place associated with the publisher.
    pub location: Option<Text>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A retained URL and the date on which it was accessed.
pub struct Url {
    /// Supplied URL text, which validation may flag as malformed.
    pub value: String,
    /// Access date at its supplied precision.
    pub accessed: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
/// Portable JSON-compatible data in a named extension namespace.
pub enum ExtensionValue {
    /// Explicit null, distinct from an absent extension field.
    Null,
    /// Boolean extension value.
    Boolean(bool),
    /// Finite number within the portable record contract's bounds.
    Number(f64),
    /// Unicode extension value.
    String(String),
    /// Ordered extension values.
    Array(Vec<ExtensionValue>),
    /// Extension properties ordered by key.
    Object(BTreeMap<String, ExtensionValue>),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
/// Complete owned bibliography data used for construction and extraction.
///
/// Nested parents describe containers and need not have independent library keys.
/// A library validates and retains these records before deriving rendering state.
pub struct EntryRecord {
    /// Library lookup key, unique among top-level records.
    pub key: String,
    /// RefKit entry-type name used when preparing rendering state.
    pub entry_type: String,
    /// Full title and any protected, mathematical, or abbreviated text.
    pub title: Option<Text>,
    /// Primary authors in source order.
    pub authors: Vec<Name>,
    /// Primary editors in source order.
    pub editors: Vec<Name>,
    /// Other contributors grouped by role.
    pub affiliated: Vec<Contributors>,
    /// Publication or issuance date.
    pub date: Option<Date>,
    /// Date of the associated event.
    pub event_date: Option<Date>,
    /// Date of original publication.
    pub original_date: Option<Date>,
    /// Publisher identity and location.
    pub publisher: Option<Publisher>,
    /// Work or event location distinct from the publisher's location.
    pub location: Option<Text>,
    /// Issuing or sponsoring organization.
    pub organization: Option<Text>,
    /// Issue designation.
    pub issue: Option<ScalarValue>,
    /// Chapter designation.
    pub chapter: Option<ScalarValue>,
    /// Volume designation for this record.
    pub volume: Option<ScalarValue>,
    /// Total number of volumes in the work.
    pub volume_total: Option<ScalarValue>,
    /// Edition designation.
    pub edition: Option<ScalarValue>,
    /// Cited page range or literal pagination.
    pub page_range: Option<ScalarValue>,
    /// Total page count of the work.
    pub page_total: Option<ScalarValue>,
    /// Cited time interval within audiovisual material.
    pub time_range: Option<ScalarValue>,
    /// Complete running time of the work.
    pub runtime: Option<ScalarValue>,
    /// URL and access date.
    pub url: Option<Url>,
    /// Identifier scheme names mapped to their retained values.
    pub identifiers: BTreeMap<String, String>,
    /// Language tag supplied for the work.
    pub language: Option<String>,
    /// Archive holding the work.
    pub archive: Option<Text>,
    /// Location within the archive.
    pub archive_location: Option<Text>,
    /// Archive or library call number.
    pub call_number: Option<Text>,
    /// Bibliographic note.
    pub note: Option<Text>,
    /// Abstract of the work.
    pub abstract_text: Option<Text>,
    /// Genre or descriptive form of the work.
    pub genre: Option<Text>,
    /// Keywords in retained order.
    pub keywords: Vec<String>,
    /// Nested containers in source order.
    pub parents: Vec<EntryRecord>,
    /// Namespace to field to portable extension value.
    pub extensions: BTreeMap<String, BTreeMap<String, ExtensionValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A record contract violation at a canonical field path.
pub struct RecordError {
    /// Field or record path identifying the rejected input.
    pub path: String,
    /// Description of the violated contract.
    pub message: String,
}

impl RecordError {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
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

/// Check record-JSON byte and structural nesting budgets before deserialization.
///
/// # Errors
/// Returns an error when source exceeds 128 MiB or 256 structural nesting levels.
/// Full JSON syntax and record shape validation follow in library construction.
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
            continue;
        }
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
    Ok(())
}
