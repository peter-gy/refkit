mod codec;
mod document;
mod duplicates;
mod library;
mod raw;
mod record;
mod references;
mod render;
mod render_tree;
mod source;
mod strings;
mod style;
pub mod tidy;
mod validation;

pub use codec::{
    BibliographyFormat, CodecError, ConversionIssue, ConversionReport, DecodeReport, EncodeReport,
    LossPolicy, convert, decode, encode,
};
pub use document::{CitationRequest, Cite, CitePurpose, Document, DocumentError, RenderedDocument};
pub use duplicates::{
    DuplicateConflict, DuplicateConflictKind, DuplicateEvidence, DuplicateGroup, DuplicateMember,
    DuplicateReport, DuplicateValue, MergeError, MergeErrorCode, MergeFieldChoice, MergePlan,
    MergeRequest,
};
pub use library::{
    Diagnostic, DiagnosticAction, DiagnosticSeverity, EntryField, EntryFieldError, Library,
    LibraryError, ParseFailure, ParseReport, RecoveryPolicy, parse_bibtex_report,
};
pub use raw::{
    BibEdit, BibEntryMapping, BibFieldMapping, BibFieldValue, BibPatchChange, BibPatchError,
    BibPatchErrorCode, BibPatchKind, BibPatchResult, BibPatchWarning, RawBlockInfo, RawDocument,
    RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo, ResolvedBibEntry,
};
pub use record::{
    Contributors, Date, DateParts, DateValue, EntryRecord, ExtensionValue, Name, Publisher,
    RecordError, ScalarValue, Text, TextChunk, TextKind, Url, validate_record_source,
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
pub use style::{
    PreparedStyle, StyleError, StyleMetadata, load_prepared_style, prepare_style_from_xml,
    style_catalog,
};
pub use tidy::{
    DuplicateRule, MergeStrategy, TidyError, TidyOptions, TidyRename, TidyResult, TidyWarning,
    tidy_bibtex,
};
pub use validation::{
    ValidationCode, ValidationIssue, ValidationProfile, ValidationReport, ValidationSeverity,
    ValidationTarget,
};
