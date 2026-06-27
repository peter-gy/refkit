use polars::prelude::*;

use refkit_core::{CoreLibrary, PreparedStyle, load_prepared_style};

pub(super) fn parse_value_library_source(
    source: &str,
    strict: bool,
) -> Result<CoreLibrary, String> {
    CoreLibrary::parse_bibtex(source, strict)
}

pub(super) fn parse_broadcast_library(bibtex: &StringChunked, strict: bool) -> Option<CoreLibrary> {
    if bibtex.len() != 1 {
        return None;
    }
    parse_value_library_source(bibtex.get(0)?, strict).ok()
}

pub(super) fn load_style(name: &str) -> PolarsResult<std::sync::Arc<PreparedStyle>> {
    load_prepared_style(name).map_err(|err| compute_error(err.to_string()))
}

pub(super) fn broadcast_len(left: usize, right: usize, operation: &str) -> PolarsResult<usize> {
    if left == right {
        return Ok(left);
    }
    if left == 1 {
        return Ok(right);
    }
    if right == 1 {
        return Ok(left);
    }
    polars_bail!(
        ComputeError:
        "{} input lengths must match or broadcast one side, got bibtex={} and key={}",
        operation,
        left,
        right
    )
}

pub(super) fn broadcast_get(values: &StringChunked, index: usize) -> Option<&str> {
    values.get(if values.len() == 1 { 0 } else { index })
}

pub(super) fn compute_error(err: String) -> PolarsError {
    PolarsError::ComputeError(err.into())
}
