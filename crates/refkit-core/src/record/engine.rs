use std::str::FromStr;

use hayagriva::types as h;

use super::{
    Contributors, Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, Publisher,
    RecordError, ScalarValue, Text, TextChunk, TextKind, Url, fmt,
};

impl Text {
    pub(crate) fn from_engine(text: &h::FormatString) -> Self {
        Self {
            chunks: chunks_from_engine(&text.value),
            short: text.short.as_deref().map(chunks_from_engine),
        }
    }

    pub(crate) fn to_engine(&self) -> h::FormatString {
        h::FormatString {
            value: chunks_to_engine(&self.chunks),
            short: self
                .short
                .as_ref()
                .map(|chunks| Box::new(chunks_to_engine(chunks))),
        }
    }
}

fn chunks_from_engine(text: &h::ChunkedString) -> Vec<TextChunk> {
    text.0
        .iter()
        .map(|chunk| TextChunk {
            text: chunk.value.clone(),
            kind: match chunk.kind {
                h::ChunkKind::Normal => TextKind::Normal,
                h::ChunkKind::Verbatim => TextKind::Protected,
                h::ChunkKind::Math => TextKind::Math,
            },
        })
        .collect()
}

fn chunks_to_engine(chunks: &[TextChunk]) -> h::ChunkedString {
    h::ChunkedString(
        chunks
            .iter()
            .map(|chunk| {
                h::StringChunk::new(
                    chunk.text.clone(),
                    match chunk.kind {
                        TextKind::Normal => h::ChunkKind::Normal,
                        TextKind::Protected => h::ChunkKind::Verbatim,
                        TextKind::Math => h::ChunkKind::Math,
                    },
                )
            })
            .collect(),
    )
}

impl Name {
    pub(crate) fn from_engine(name: &h::Person) -> Self {
        Self::Person {
            family: name.name.clone(),
            given: name.given_name.clone(),
            prefix: name.prefix.clone(),
            suffix: name.suffix.clone(),
            alias: name.alias.clone(),
            comma_suffix: name.comma_suffix,
            id: None,
            given_initials: None,
            prefix_initials: None,
            use_prefix: None,
            non_dropping_particle: None,
        }
    }

    pub(crate) fn to_engine(&self) -> h::Person {
        match self {
            Self::Person {
                family,
                given,
                prefix,
                suffix,
                alias,
                comma_suffix,
                non_dropping_particle,
                ..
            } => h::Person {
                name: non_dropping_particle
                    .as_ref()
                    .map_or_else(|| family.clone(), |particle| format!("{particle} {family}")),
                given_name: given.clone(),
                prefix: prefix.clone(),
                suffix: suffix.clone(),
                alias: alias.clone(),
                comma_suffix: *comma_suffix,
            },
            Self::Organization { name } => h::Person {
                name: name.clone(),
                given_name: None,
                prefix: None,
                suffix: None,
                alias: None,
                comma_suffix: false,
            },
        }
    }
}

impl Date {
    pub(crate) fn from_engine(date: &h::Date) -> Self {
        Self {
            value: DateValue::Point {
                date: DateParts {
                    year: date.year,
                    month: date.month.map(|month| month + 1),
                    day: date.day.map(|day| day + 1),
                    season: date
                        .season
                        .map(hayagriva::citationberg::taxonomy::Season::to_csl_number),
                    time: None,
                },
            },
            uncertain: false,
            approximate: date.approximate,
        }
    }

    pub(crate) fn to_engine(&self, path: &str) -> Result<Option<h::Date>, RecordError> {
        let date = match &self.value {
            DateValue::Point { date } => {
                validate_date_parts(date, path)?;
                Some(date)
            }
            DateValue::Range { start, end } => {
                if let Some(start) = start {
                    validate_date_parts(start, path)?;
                }
                if let Some(end) = end {
                    validate_date_parts(end, path)?;
                }
                if start.is_none() && end.is_none() {
                    return Err(RecordError::new(
                        path,
                        "date range needs at least one endpoint",
                    ));
                }
                end.as_ref().or(start.as_ref())
            }
            DateValue::Literal { .. } => None,
        };
        Ok(date.map(|date| h::Date {
            year: date.year,
            month: date.month.map(|month| month - 1),
            day: date.day.map(|day| day - 1),
            approximate: self.approximate || self.uncertain,
            season: date.season.and_then(|value| {
                hayagriva::citationberg::taxonomy::Season::try_from_csl_number(value).ok()
            }),
        }))
    }
}

fn validate_date_parts(date: &DateParts, path: &str) -> Result<(), RecordError> {
    let leap = date.year.rem_euclid(4) == 0
        && (date.year.rem_euclid(100) != 0 || date.year.rem_euclid(400) == 0);
    let max_day = match date.month {
        Some(2) => {
            if leap {
                29
            } else {
                28
            }
        }
        Some(4 | 6 | 9 | 11) => 30,
        Some(1 | 3 | 5 | 7 | 8 | 10 | 12) => 31,
        Some(_) => return Err(RecordError::new(path, "month must be between 1 and 12")),
        None => 0,
    };
    if date.day.is_some_and(|day| day == 0 || day > max_day) {
        return Err(RecordError::new(
            path,
            "day must be valid for the supplied year and month",
        ));
    }
    if date.season.is_some_and(|season| !(1..=4).contains(&season))
        || (date.season.is_some() && date.month.is_some())
    {
        return Err(RecordError::new(
            path,
            "season must be 1 through 4 and cannot accompany a month",
        ));
    }
    if let Some(time) = &date.time {
        let (Some(month), Some(day)) = (date.month, date.day) else {
            return Err(RecordError::new(
                path,
                "time requires a complete calendar date",
            ));
        };
        let value = format!("{}-{month:02}-{day:02}T{time}", date.year_text());
        Date::parse_biblatex(&value).map_err(|error| RecordError::new(path, error))?;
    }
    Ok(())
}

impl ScalarValue {
    fn from_engine<T: fmt::Display>(value: &h::MaybeTyped<T>) -> Self {
        match value {
            h::MaybeTyped::Typed(value) => Self::Typed(value.to_string()),
            h::MaybeTyped::String(value) => Self::Literal(value.clone()),
        }
    }

    fn to_engine<T: FromStr>(&self, path: &str) -> Result<h::MaybeTyped<T>, RecordError>
    where
        T::Err: fmt::Display,
    {
        match self {
            Self::Typed(value) => value
                .parse::<T>()
                .map(h::MaybeTyped::Typed)
                .map_err(|error| RecordError::new(path, error.to_string())),
            Self::Literal(value) => Ok(h::MaybeTyped::String(value.clone())),
        }
    }
}

fn role_name(role: &h::PersonRole) -> String {
    match role {
        h::PersonRole::Unknown(name) => name.clone(),
        _ => serde_json::to_value(role)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default(),
    }
}

impl EntryRecord {
    pub(crate) fn validate_limits(
        &self,
        depth: usize,
        visited: &mut usize,
    ) -> Result<(), RecordError> {
        *visited += 1;
        if depth > 64 || *visited > 100_000 {
            return Err(RecordError::new(
                &self.key,
                "record graph exceeds resource limits",
            ));
        }
        let mut extension_nodes = 0;
        for (namespace, fields) in &self.extensions {
            for (field, value) in fields {
                validate_extension(
                    value,
                    0,
                    &mut extension_nodes,
                    &format!("{}.extensions.{namespace}.{field}", self.key),
                )?;
            }
        }
        for parent in &self.parents {
            parent.validate_limits(depth + 1, visited)?;
        }
        Ok(())
    }

    pub(crate) fn from_engine(entry: &hayagriva::Entry) -> Self {
        Self {
            key: entry.key().to_string(),
            entry_type: crate::strings::entry_type_name(*entry.entry_type()).to_string(),
            title: entry.title().map(Text::from_engine),
            authors: entry
                .authors()
                .unwrap_or_default()
                .iter()
                .map(Name::from_engine)
                .collect(),
            editors: entry
                .editors()
                .unwrap_or_default()
                .iter()
                .map(Name::from_engine)
                .collect(),
            affiliated: entry
                .affiliated()
                .unwrap_or_default()
                .iter()
                .map(|group| Contributors {
                    role: role_name(&group.role),
                    names: group.names.iter().map(Name::from_engine).collect(),
                })
                .collect(),
            date: entry.date().map(Date::from_engine),
            publisher: entry.publisher().map(|publisher| Publisher {
                name: publisher.name().map(Text::from_engine),
                location: publisher.location().map(Text::from_engine),
            }),
            location: entry.location().map(Text::from_engine),
            organization: entry.organization().map(Text::from_engine),
            issue: entry.issue().map(ScalarValue::from_engine),
            chapter: entry.chapter().map(ScalarValue::from_engine),
            volume: entry.volume().map(ScalarValue::from_engine),
            volume_total: entry
                .volume_total()
                .map(|value| ScalarValue::Typed(value.to_string())),
            edition: entry.edition().map(ScalarValue::from_engine),
            page_range: entry.page_range().map(ScalarValue::from_engine),
            page_total: entry
                .page_total()
                .map(|value| ScalarValue::Typed(value.to_string())),
            time_range: entry.time_range().map(ScalarValue::from_engine),
            runtime: entry.runtime().map(ScalarValue::from_engine),
            url: entry.url().map(|url| Url {
                value: url.value.to_string(),
                accessed: url.visit_date.as_ref().map(Date::from_engine),
            }),
            identifiers: entry
                .serial_number()
                .map(|identifiers| identifiers.0.clone())
                .unwrap_or_default(),
            language: entry.language().map(ToString::to_string),
            archive: entry.archive().map(Text::from_engine),
            archive_location: entry.archive_location().map(Text::from_engine),
            call_number: entry.call_number().map(Text::from_engine),
            note: entry.note().map(Text::from_engine),
            abstract_text: entry.abstract_().map(Text::from_engine),
            genre: entry.genre().map(Text::from_engine),
            parents: entry.parents().iter().map(Self::from_engine).collect(),
            ..Self::default()
        }
    }

    pub(crate) fn to_engine(&self) -> Result<hayagriva::Entry, RecordError> {
        self.prepare_engine(0, &mut 0)
    }

    fn prepare_engine(
        &self,
        depth: usize,
        visited: &mut usize,
    ) -> Result<hayagriva::Entry, RecordError> {
        *visited += 1;
        if depth > 64 || *visited > 100_000 {
            return Err(RecordError::new(
                &self.key,
                "record graph exceeds resource limits",
            ));
        }
        let mut entry = prepare_identity(self)?;
        prepare_text_fields(self, &mut entry);
        prepare_creators(self, &mut entry);
        prepare_dates(self, &mut entry)?;
        if let Some(publisher) = &self.publisher {
            entry.set_publisher(h::Publisher::new(
                publisher.name.as_ref().map(Text::to_engine),
                publisher.location.as_ref().map(Text::to_engine),
            ));
        }
        prepare_numbers(self, &mut entry)?;
        prepare_links(self, &mut entry)?;
        entry.set_parents(prepare_parents(self, depth, visited)?);
        Ok(entry)
    }
}

fn prepare_identity(record: &EntryRecord) -> Result<hayagriva::Entry, RecordError> {
    let mut extension_nodes = 0;
    for (namespace, fields) in &record.extensions {
        for (field, value) in fields {
            validate_extension(
                value,
                0,
                &mut extension_nodes,
                &format!("{}.extensions.{namespace}.{field}", record.key),
            )?;
        }
    }
    let kind = serde_json::from_value::<h::EntryType>(serde_json::Value::String(
        record.entry_type.clone(),
    ))
    .map_err(|_| RecordError::new(format!("{}.entry_type", record.key), "unknown entry type"))?;
    if crate::strings::entry_type_name(kind) != record.entry_type {
        return Err(RecordError::new(
            format!("{}.entry_type", record.key),
            "entry type must use its canonical TitleCase name",
        ));
    }
    if record.key.is_empty() {
        return Err(RecordError::new("key", "entry key must not be empty"));
    }
    Ok(hayagriva::Entry::new(&record.key, kind))
}

fn prepare_text_fields(record: &EntryRecord, entry: &mut hayagriva::Entry) {
    macro_rules! text_fields {
        ($($field:ident => $setter:ident),* $(,)?) => { $(if let Some(value) = &record.$field { entry.$setter(value.to_engine()); })* };
    }
    text_fields!(title => set_title, location => set_location, organization => set_organization, archive => set_archive, archive_location => set_archive_location, call_number => set_call_number, note => set_note, abstract_text => set_abstract_, genre => set_genre);
}

fn prepare_creators(record: &EntryRecord, entry: &mut hayagriva::Entry) {
    if !record.authors.is_empty() {
        entry.set_authors(record.authors.iter().map(Name::to_engine).collect());
    }
    if !record.editors.is_empty() {
        entry.set_editors(record.editors.iter().map(Name::to_engine).collect());
    }
    if !record.affiliated.is_empty() {
        entry.set_affiliated(
            record
                .affiliated
                .iter()
                .map(|group| {
                    h::PersonsWithRoles::new(
                        group.names.iter().map(Name::to_engine).collect(),
                        serde_json::from_value(serde_json::Value::String(group.role.clone()))
                            .unwrap_or_else(|_| h::PersonRole::Unknown(group.role.clone())),
                    )
                })
                .collect(),
        );
    }
}

fn prepare_dates(record: &EntryRecord, entry: &mut hayagriva::Entry) -> Result<(), RecordError> {
    for (field, date) in [
        ("date", &record.date),
        ("event_date", &record.event_date),
        ("original_date", &record.original_date),
    ] {
        if let Some(date) = date {
            let prepared = date.to_engine(&format!("{}.{field}", record.key))?;
            if field == "date"
                && let Some(date) = prepared
            {
                entry.set_date(date);
            }
        }
    }
    Ok(())
}

fn prepare_numbers(record: &EntryRecord, entry: &mut hayagriva::Entry) -> Result<(), RecordError> {
    macro_rules! scalar_fields {
        ($($field:ident => $setter:ident),* $(,)?) => { $(if let Some(value) = &record.$field { entry.$setter(value.to_engine(&format!("{}.{}", record.key, stringify!($field)))?); })* };
    }
    scalar_fields!(issue => set_issue, chapter => set_chapter, volume => set_volume, edition => set_edition, page_range => set_page_range, time_range => set_time_range, runtime => set_runtime);
    for (field, value) in [
        ("volume_total", &record.volume_total),
        ("page_total", &record.page_total),
    ] {
        if let Some(value) = value {
            let h::MaybeTyped::Typed(number) =
                value.to_engine::<h::Numeric>(&format!("{}.{field}", record.key))?
            else {
                return Err(RecordError::new(
                    format!("{}.{field}", record.key),
                    "total requires a typed number",
                ));
            };
            if field == "volume_total" {
                entry.set_volume_total(number);
            } else {
                entry.set_page_total(number);
            }
        }
    }
    Ok(())
}

fn prepare_links(record: &EntryRecord, entry: &mut hayagriva::Entry) -> Result<(), RecordError> {
    if let Some(url) = &record.url {
        let accessed = url
            .accessed
            .as_ref()
            .map(|date| date.to_engine(&format!("{}.url.accessed", record.key)))
            .transpose()?
            .flatten();
        if let Ok(mut prepared) = url.value.parse::<h::QualifiedUrl>() {
            prepared.visit_date = accessed;
            entry.set_url(prepared);
        }
    }
    if !record.identifiers.is_empty() {
        entry.set_serial_number(h::SerialNumber(record.identifiers.clone()));
    }
    if let Some(language) = &record.language {
        entry.set_language(language.parse().map_err(|error| {
            RecordError::new(format!("{}.language", record.key), format!("{error}"))
        })?);
    }
    Ok(())
}

fn prepare_parents(
    record: &EntryRecord,
    depth: usize,
    visited: &mut usize,
) -> Result<Vec<hayagriva::Entry>, RecordError> {
    let mut parents: Vec<_> = record
        .parents
        .iter()
        .map(|parent| parent.prepare_engine(depth + 1, visited))
        .collect::<Result<_, _>>()?;
    for (kind, date, field) in [
        (
            h::EntryType::Original,
            &record.original_date,
            "original_date",
        ),
        (h::EntryType::Conference, &record.event_date, "event_date"),
    ] {
        if let Some(date) = date
            && let Some(date) = date.to_engine(&format!("{}.{field}", record.key))?
        {
            if let Some(parent) = parents
                .iter_mut()
                .find(|parent| parent.entry_type() == &kind)
            {
                parent.set_date(date);
            } else {
                let mut parent = hayagriva::Entry::new(&record.key, kind);
                parent.set_date(date);
                parents.push(parent);
            }
        }
    }
    Ok(parents)
}

fn validate_extension(
    value: &ExtensionValue,
    depth: usize,
    visited: &mut usize,
    path: &str,
) -> Result<(), RecordError> {
    *visited += 1;
    if depth > 64 || *visited > 100_000 {
        return Err(RecordError::new(path, "extension exceeds resource limits"));
    }
    match value {
        ExtensionValue::Number(value) if !value.is_finite() => {
            return Err(RecordError::new(path, "extension numbers must be finite"));
        }
        ExtensionValue::Number(value)
            if value.fract() == 0.0 && value.abs() > 9_007_199_254_740_991.0 =>
        {
            return Err(RecordError::new(
                path,
                "integer extension values must be exactly representable in JavaScript",
            ));
        }
        ExtensionValue::Array(values) => {
            for value in values {
                validate_extension(value, depth + 1, visited, path)?;
            }
        }
        ExtensionValue::Object(values) => {
            for value in values.values() {
                validate_extension(value, depth + 1, visited, path)?;
            }
        }
        _ => {}
    }
    Ok(())
}
