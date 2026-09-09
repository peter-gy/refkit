mod document;
mod library;
mod raw;
mod render;
mod render_tree;
mod source;
mod strings;
mod style;
pub mod tidy;

pub use document::{CitationRequest, Cite, Document, DocumentError, RenderedDocument};
pub use library::{
    Diagnostic, DiagnosticAction, DiagnosticSeverity, EntryField, EntryFieldError, EntryRecord,
    Library, LibraryError, ParseFailure, ParseReport, RecoveryPolicy, parse_bibtex_report,
};
pub use raw::{
    RawBlockInfo, RawDocument, RawEditError, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo,
    ResolvedBibEntry,
};
pub use render::{
    RenderedOutput, is_bundled_locale, render_library_bibliography, render_library_citation,
    render_library_citation_each, render_library_citation_group,
};
pub use render_tree::{
    BibliographyLayout, FontStyle, FontVariant, FontWeight, RenderedDisplay, RenderedFormatting,
    RenderedMeta, RenderedNode, RenderedRecord, SecondFieldAlign, TextDecoration, VerticalAlign,
};
pub use source::{DecodedText, TextEncoding, decode_bibliography};
pub(crate) use strings::quoted;
pub use style::{PreparedStyle, StyleError, load_prepared_style, prepare_style_from_xml};
pub use tidy::{
    DuplicateRule, MergeStrategy, TidyError, TidyOptions, TidyRename, TidyResult, TidyWarning,
    tidy_bibtex,
};
