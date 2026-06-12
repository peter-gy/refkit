mod document;
mod library;
mod raw;
mod render;
mod render_tree;
mod strings;
mod style;
mod style_analysis;

pub use document::{Cite as CoreCite, Document as CoreDocument, DocumentError};
pub use library::{
    EntryRecord, Library as CoreLibrary, NormalizedEntry, NormalizedValue, ParseReport,
    ProjectField, SourceText, parse_bibtex_report_source, parse_project_field,
    read_bibliography_text,
};
pub use raw::{
    RawBlockInfo, RawDocument, RawEditError, RawEntryId, RawEntryInfo, RawFieldId, RawFieldInfo,
};
pub use render::{
    RenderedOutput, bundled_locales, render_library_bibliography, render_library_citation,
    render_library_citation_sequence,
};
pub use render_tree::{RenderedFormatting, RenderedNode, RenderedRecord};
pub use strings::{
    display_name, elem_meta_name, entry_type_name, font_style_name, font_variant_name,
    font_weight_name, formatting_summary, option_quoted, quoted, text_decoration_name,
    vertical_align_name,
};
pub use style::{PreparedStyle, StyleError, load_prepared_style, prepare_style_from_xml};
