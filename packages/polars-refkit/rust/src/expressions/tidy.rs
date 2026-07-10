use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use pyo3_polars::export::polars_arrow::array::IntoBoxedArray;
use pyo3_polars::export::polars_arrow::bitmap::Bitmap;
use refkit_core::{
    DuplicateRule, MergeStrategy, TidyOptions, TidyWarning, tidy_bibtex as core_tidy_bibtex,
};

use super::broadcast::compute_error;
use super::dtypes::{string_output, tidy_report_output, tidy_warning_struct_dtype};
use super::{DefaultableString, DefaultableStringList, DefaultableUsize, TidyKwargs};

#[derive(Debug, Clone)]
struct TidyReportRow {
    ok: bool,
    bibtex: Option<String>,
    count: Option<u32>,
    warnings: Option<Vec<TidyWarning>>,
    error: Option<String>,
}

#[polars_expr(output_type_func=string_output)]
fn tidy_bibtex(inputs: &[Series], kwargs: TidyKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let options = kwargs.to_options().map_err(compute_error)?;
    let output = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            core_tidy_bibtex(source, options.clone())
                .ok()
                .map(|result| result.bibtex)
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type_func=tidy_report_output)]
fn tidy_bibtex_report(inputs: &[Series], kwargs: TidyKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let options = kwargs.to_options().map_err(compute_error)?;
    let reports = bibtex
        .iter()
        .map(|value| value.map(|source| tidy_source_report(source, options.clone())))
        .collect::<Vec<_>>();
    tidy_reports_to_struct_series("tidy_bibtex_report", &reports)
}

impl TidyKwargs {
    fn to_options(&self) -> Result<TidyOptions, String> {
        let mut options = TidyOptions::default();

        if let Some(value) = &self.omit {
            options.omit = value.clone();
        }
        if let Some(value) = self.curly {
            options.curly = value;
        }
        if let Some(value) = self.numeric {
            options.numeric = value;
        }
        if let Some(value) = self.months {
            options.months = value;
        }
        if let Some(value) = self.space {
            options.space = value;
        }
        if let Some(value) = self.tab {
            options.tab = value;
        }
        if let Some(value) = &self.align {
            options.align = value.option_usize(TidyOptions::default().align);
        }
        if let Some(value) = self.blank_lines {
            options.blank_lines = value;
        }
        if let Some(value) = &self.sort {
            options.sort = value.option_string_list(TidyOptions::default().with_sort().sort);
        }
        if let Some(value) = &self.duplicates {
            options.duplicates = Some(duplicate_rules(value)?);
        }
        if let Some(value) = &self.merge {
            options.merge = Some(merge_strategy(value)?);
        }
        if let Some(value) = self.strip_enclosing_braces {
            options.strip_enclosing_braces = value;
        }
        if let Some(value) = self.drop_all_caps {
            options.drop_all_caps = value;
        }
        if let Some(value) = self.escape {
            options.escape = value;
        }
        if let Some(value) = &self.sort_fields {
            options.sort_fields =
                value.option_string_list(TidyOptions::default().with_sort_fields().sort_fields);
        }
        if let Some(value) = self.strip_comments {
            options.strip_comments = value;
        }
        if let Some(value) = self.trailing_commas {
            options.trailing_commas = value;
        }
        if let Some(value) = self.encode_urls {
            options.encode_urls = value;
        }
        if let Some(value) = self.tidy_comments {
            options.tidy_comments = value;
        }
        if let Some(value) = self.remove_empty_fields {
            options.remove_empty_fields = value;
        }
        if let Some(value) = self.remove_duplicate_fields {
            options.remove_duplicate_fields = value;
        }
        if let Some(value) = &self.generate_keys {
            options.generate_keys =
                value.option_string(TidyOptions::default().with_generate_keys().generate_keys);
        }
        if let Some(value) = self.max_authors {
            options.max_authors = Some(value);
        }
        if let Some(value) = self.lowercase {
            options.lowercase = value;
        }
        if let Some(value) = &self.enclosing_braces {
            options.enclosing_braces = value.option_string_list(
                TidyOptions::default()
                    .with_enclosing_braces()
                    .enclosing_braces,
            );
        }
        if let Some(value) = &self.remove_braces {
            options.remove_braces =
                value.option_string_list(TidyOptions::default().with_remove_braces().remove_braces);
        }
        if let Some(value) = &self.wrap {
            options.wrap = value.option_usize(TidyOptions::default().with_wrap().wrap);
        }

        Ok(options)
    }
}

impl DefaultableUsize {
    fn option_usize(&self, default: Option<usize>) -> Option<usize> {
        match self {
            Self::Enabled(true) => default,
            Self::Enabled(false) => None,
            Self::Value(value) => Some(*value),
        }
    }
}

impl DefaultableString {
    fn option_string(&self, default: Option<String>) -> Option<String> {
        match self {
            Self::Enabled(true) => default,
            Self::Enabled(false) => None,
            Self::Value(value) => Some(value.clone()),
        }
    }
}

impl DefaultableStringList {
    fn option_string_list(&self, default: Option<Vec<String>>) -> Option<Vec<String>> {
        match self {
            Self::Enabled(true) => default,
            Self::Enabled(false) => None,
            Self::Values(values) => Some(values.clone()),
        }
    }
}

fn duplicate_rules(values: &[String]) -> Result<Vec<DuplicateRule>, String> {
    values
        .iter()
        .map(|value| match value.as_str() {
            "doi" => Ok(DuplicateRule::Doi),
            "key" => Ok(DuplicateRule::Key),
            "abstract" => Ok(DuplicateRule::Abstract),
            "citation" => Ok(DuplicateRule::Citation),
            _ => Err(format!("unknown duplicate rule {value:?}")),
        })
        .collect()
}

fn merge_strategy(value: &str) -> Result<MergeStrategy, String> {
    match value {
        "first" => Ok(MergeStrategy::First),
        "last" => Ok(MergeStrategy::Last),
        "combine" => Ok(MergeStrategy::Combine),
        "overwrite" => Ok(MergeStrategy::Overwrite),
        _ => Err(format!("unknown merge strategy {value:?}")),
    }
}

fn tidy_source_report(source: &str, options: TidyOptions) -> TidyReportRow {
    match core_tidy_bibtex(source, options) {
        Ok(result) => TidyReportRow {
            ok: true,
            bibtex: Some(result.bibtex),
            count: u32::try_from(result.count).ok(),
            warnings: Some(result.warnings),
            error: None,
        },
        Err(err) => TidyReportRow {
            ok: false,
            bibtex: None,
            count: None,
            warnings: None,
            error: Some(err.to_string()),
        },
    }
}

fn tidy_reports_to_struct_series(
    name: &str,
    reports: &[Option<TidyReportRow>],
) -> PolarsResult<Series> {
    let ok = BooleanChunked::from_iter_options(
        "ok".into(),
        reports
            .iter()
            .map(|report| report.as_ref().map(|row| row.ok)),
    )
    .into_series();
    let bibtex = StringChunked::from_iter_options(
        "bibtex".into(),
        reports
            .iter()
            .map(|report| report.as_ref().and_then(|row| row.bibtex.as_deref())),
    )
    .into_series();
    let count = UInt32Chunked::from_iter_options(
        "count".into(),
        reports
            .iter()
            .map(|report| report.as_ref().and_then(|row| row.count)),
    )
    .into_series();
    let warnings = warning_lists_to_series(reports)?;
    let error = StringChunked::from_iter_options(
        "error".into(),
        reports
            .iter()
            .map(|report| report.as_ref().and_then(|row| row.error.as_deref())),
    )
    .into_series();
    let fields = [ok, bibtex, count, warnings, error];
    let chunked = StructChunked::from_series(name.into(), reports.len(), fields.iter())?;
    if reports.iter().all(Option::is_some) {
        return Ok(chunked.into_series());
    }

    let validity = Bitmap::from_iter(reports.iter().map(Option::is_some));
    let chunks = chunked
        .downcast_iter()
        .map(|array| {
            array
                .clone()
                .with_validity(Some(validity.clone()))
                .into_boxed()
        })
        .collect::<Vec<_>>();
    Ok(unsafe {
        StructChunked::from_chunks_and_dtype(
            chunked.name().clone(),
            chunks,
            chunked.dtype().clone(),
        )
    }
    .into_series())
}

fn warning_lists_to_series(reports: &[Option<TidyReportRow>]) -> PolarsResult<Series> {
    let mut builder = AnonymousOwnedListBuilder::new(
        "warnings".into(),
        reports.len(),
        Some(tidy_warning_struct_dtype()),
    );

    for report in reports {
        let Some(warnings) = report.as_ref().and_then(|row| row.warnings.as_ref()) else {
            builder.append_null();
            continue;
        };
        let warnings = warnings_to_struct_series(warnings)?;
        builder.append_series(&warnings)?;
    }

    Ok(builder.finish().into_series())
}

fn warnings_to_struct_series(warnings: &[TidyWarning]) -> PolarsResult<Series> {
    let code =
        StringChunked::from_iter_values("code".into(), warnings.iter().map(TidyWarning::code))
            .into_series();
    let rule = StringChunked::from_iter_options(
        "rule".into(),
        warnings
            .iter()
            .map(|warning| warning.rule().map(|rule| rule.as_str())),
    )
    .into_series();
    let message = StringChunked::from_iter_values(
        "message".into(),
        warnings.iter().map(TidyWarning::message),
    )
    .into_series();
    let fields = [code, rule, message];
    StructChunked::from_series("warning".into(), warnings.len(), fields.iter())
        .map(|warnings| warnings.into_series())
}
