use std::fs;
use std::path::Path;

use pyo3::prelude::*;
use refkit_core::{TextEncoding, decode_bibliography};

use crate::errors::RefkitError;
use crate::repr::quoted;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LibraryFormat {
    Biblatex,
    HayagrivaYaml,
}

pub(crate) struct LibrarySource {
    pub(crate) text: String,
    pub(crate) format: LibraryFormat,
    pub(crate) diagnostic: Option<String>,
}

pub(crate) fn read_library(path: &Path) -> Result<LibrarySource, String> {
    let decoded = read_bibliography(path)?;
    let format = match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bib") => LibraryFormat::Biblatex,
        Some("yaml" | "yml") => LibraryFormat::HayagrivaYaml,
        Some(extension) => {
            return Err(format!(
                "unsupported bibliography extension {}",
                quoted(extension)
            ));
        }
        None => return Err("bibliography path has no extension".to_string()),
    };
    let diagnostic = (decoded.encoding == TextEncoding::Windows1252).then(|| {
        format!(
            "decoded {} as Windows-1252-compatible text because it is not valid UTF-8",
            path.display()
        )
    });
    Ok(LibrarySource {
        text: decoded.text,
        format,
        diagnostic,
    })
}

pub(crate) fn read_bibtex(path: &Path) -> Result<String, String> {
    read_bibliography(path).map(|decoded| decoded.text)
}

pub(crate) fn read_style(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("failed to read style: {err}"))
}

pub(crate) fn write_bibtex(path: &Path, source: &str) -> Result<(), String> {
    fs::write(path, source).map_err(|err| format!("failed to write BibTeX: {err}"))
}

#[pyfunction(name = "_write_bibtex")]
pub(crate) fn write_bibtex_py(
    py: Python<'_>,
    path: std::path::PathBuf,
    source: String,
) -> PyResult<()> {
    py.detach(move || write_bibtex(&path, &source))
        .map_err(RefkitError::new_err)
}

fn read_bibliography(path: &Path) -> Result<refkit_core::DecodedText, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    Ok(decode_bibliography(&bytes))
}
