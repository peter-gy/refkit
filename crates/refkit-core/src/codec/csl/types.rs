use crate::EntryRecord;

pub(in crate::codec) fn canonical_type(record: &EntryRecord) -> &'static str {
    let parent = record
        .parents
        .iter()
        .find(|parent| !["Original", "Conference"].contains(&parent.entry_type.as_str()))
        .or_else(|| {
            record
                .parents
                .iter()
                .find(|parent| parent.entry_type == "Conference")
        });
    match (
        record.entry_type.as_str(),
        parent.map(|parent| parent.entry_type.as_str()),
    ) {
        ("Article", Some("Periodical")) => "article-journal",
        ("Article", Some("Newspaper")) => "article-newspaper",
        ("Article", Some("Proceedings" | "Conference")) => "paper-conference",
        ("Article", _) => "article",
        ("Chapter" | "Anthos", _) => "chapter",
        ("Entry", _) => "entry-encyclopedia",
        ("Book" | "Anthology" | "Reference", _) => "book",
        ("Report", _) => "report",
        ("Thesis", _) => "thesis",
        ("Web", _) => "webpage",
        ("Post", Some("Blog")) => "post-weblog",
        ("Post", _) => "post",
        ("Repository", _) => "software",
        ("Patent", _) => "patent",
        ("Case", _) => "legal_case",
        ("Legislation", _) => "legislation",
        ("Manuscript", _) => "manuscript",
        ("Video" | "Scene", _) => "motion_picture",
        ("Audio", _) => "song",
        ("Artwork", _) => "graphic",
        ("Performance", _) => "speech",
        _ => "document",
    }
}

pub(in crate::codec) fn record_type(kind: &str) -> Option<(&'static str, Option<&'static str>)> {
    Some(match kind {
        "article-journal" | "article-magazine" => ("Article", Some("Periodical")),
        "article-newspaper" => ("Article", Some("Newspaper")),
        "paper-conference" => ("Article", Some("Proceedings")),
        "article" => ("Article", None),
        "chapter" => ("Chapter", Some("Book")),
        "entry-dictionary" | "entry-encyclopedia" => ("Entry", Some("Reference")),
        "book" => ("Book", None),
        "report" => ("Report", None),
        "thesis" => ("Thesis", None),
        "webpage" => ("Web", None),
        "post" => ("Post", None),
        "post-weblog" => ("Post", Some("Blog")),
        "software" | "dataset" => ("Repository", None),
        "patent" => ("Patent", None),
        "legal_case" => ("Case", None),
        "legislation" => ("Legislation", None),
        "manuscript" => ("Manuscript", None),
        "motion_picture" => ("Video", None),
        "song" => ("Audio", None),
        "graphic" => ("Artwork", None),
        "speech" => ("Performance", None),
        "document" | "pamphlet" | "personal_communication" => ("Misc", None),
        _ => return None,
    })
}
