mod library;
mod render;
mod strings;
mod style_analysis;

pub use hayagriva::citationberg::IndependentStyle;
pub use library::{
    EntryRecord, ParsedLibrary, ProjectField, SourceText, entry_record, library_to_normalized_json,
    parse_library_path, parse_library_source, parse_project_field, project_records,
    read_bibliography_text,
};
pub use render::{
    RenderedOutput, bibliography_to_text_html, bundled_locales, elem_children_to_html,
    elem_children_to_string, load_independent_style, render_bibliography, render_citation,
    render_citation_sequence, render_independent_citation, safe_href,
};
pub use strings::{
    display_name, elem_meta_name, entry_type_name, font_style_name, font_variant_name,
    font_weight_name, formatting_summary, option_quoted, quoted, text_decoration_name,
    vertical_align_name,
};
pub use style_analysis::{
    can_fast_render_single_citations, citation_depends_on_subsequent_names, citation_only_style,
    full_history_citation_style,
};
