use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyModule};

use crate::errors::{TidyError as PyTidyError, TidySyntaxError};
use refkit_core::{
    DuplicateRule, MergeStrategy, TidyError as CoreTidyError, TidyOptions as CoreTidyOptions,
    TidyResult as CoreTidyResult, TidyWarning as CoreTidyWarning, option_quoted, quoted,
    tidy_bibtex as core_tidy_bibtex,
};

#[pyclass(module = "refkit_core", skip_from_py_object)]
#[derive(Clone)]
pub struct TidyOptions {
    inner: CoreTidyOptions,
}

#[pymethods]
impl TidyOptions {
    #[new]
    #[pyo3(
        signature = (**kwargs),
        text_signature = "(*, omit=None, curly=False, numeric=False, months=False, space=2, tab=False, align=14, blank_lines=False, sort=None, duplicates=None, merge=None, strip_enclosing_braces=False, drop_all_caps=False, escape=True, sort_fields=None, strip_comments=False, trailing_commas=False, encode_urls=False, tidy_comments=True, remove_empty_fields=False, remove_duplicate_fields=True, generate_keys=None, max_authors=None, lowercase=True, enclosing_braces=None, remove_braces=None, wrap=None)"
    )]
    fn new(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let mut options = CoreTidyOptions::default();
        let Some(kwargs) = kwargs else {
            return Ok(Self { inner: options });
        };

        for key in kwargs.keys() {
            let key = key.extract::<String>()?;
            if !TIDY_OPTION_NAMES.contains(&key.as_str()) {
                return Err(PyValueError::new_err(format!(
                    "unknown tidy option {}",
                    quoted(&key)
                )));
            }
        }

        if let Some(value) = kwargs.get_item("omit")? {
            options.omit = string_list(&value, "omit")?;
        }
        if let Some(value) = kwargs.get_item("curly")? {
            options.curly = bool_value(&value, "curly")?;
        }
        if let Some(value) = kwargs.get_item("numeric")? {
            options.numeric = bool_value(&value, "numeric")?;
        }
        if let Some(value) = kwargs.get_item("months")? {
            options.months = bool_value(&value, "months")?;
        }
        if let Some(value) = kwargs.get_item("space")? {
            options.space = usize_value(&value, "space")?;
        }
        if let Some(value) = kwargs.get_item("tab")? {
            options.tab = bool_value(&value, "tab")?;
        }
        if let Some(value) = kwargs.get_item("align")? {
            options.align = defaultable_usize(&value, "align", CoreTidyOptions::default().align)?;
        }
        if let Some(value) = kwargs.get_item("blank_lines")? {
            options.blank_lines = bool_value(&value, "blank_lines")?;
        }
        if let Some(value) = kwargs.get_item("sort")? {
            options.sort = defaultable_string_list(&value, "sort", default_sort_fields())?;
        }
        if let Some(value) = kwargs.get_item("duplicates")? {
            options.duplicates = duplicate_rules(&value, "duplicates")?;
        }
        if let Some(value) = kwargs.get_item("merge")? {
            options.merge = merge_strategy(&value, "merge")?;
        }
        if let Some(value) = kwargs.get_item("strip_enclosing_braces")? {
            options.strip_enclosing_braces = bool_value(&value, "strip_enclosing_braces")?;
        }
        if let Some(value) = kwargs.get_item("drop_all_caps")? {
            options.drop_all_caps = bool_value(&value, "drop_all_caps")?;
        }
        if let Some(value) = kwargs.get_item("escape")? {
            options.escape = bool_value(&value, "escape")?;
        }
        if let Some(value) = kwargs.get_item("sort_fields")? {
            options.sort_fields =
                defaultable_string_list(&value, "sort_fields", default_field_sort())?;
        }
        if let Some(value) = kwargs.get_item("strip_comments")? {
            options.strip_comments = bool_value(&value, "strip_comments")?;
        }
        if let Some(value) = kwargs.get_item("trailing_commas")? {
            options.trailing_commas = bool_value(&value, "trailing_commas")?;
        }
        if let Some(value) = kwargs.get_item("encode_urls")? {
            options.encode_urls = bool_value(&value, "encode_urls")?;
        }
        if let Some(value) = kwargs.get_item("tidy_comments")? {
            options.tidy_comments = bool_value(&value, "tidy_comments")?;
        }
        if let Some(value) = kwargs.get_item("remove_empty_fields")? {
            options.remove_empty_fields = bool_value(&value, "remove_empty_fields")?;
        }
        if let Some(value) = kwargs.get_item("remove_duplicate_fields")? {
            options.remove_duplicate_fields = bool_value(&value, "remove_duplicate_fields")?;
        }
        if let Some(value) = kwargs.get_item("generate_keys")? {
            options.generate_keys = generate_keys(&value, "generate_keys")?;
        }
        if let Some(value) = kwargs.get_item("max_authors")? {
            options.max_authors = optional_usize(&value, "max_authors")?;
        }
        if let Some(value) = kwargs.get_item("lowercase")? {
            options.lowercase = bool_value(&value, "lowercase")?;
        }
        if let Some(value) = kwargs.get_item("enclosing_braces")? {
            options.enclosing_braces =
                defaultable_string_list(&value, "enclosing_braces", vec!["title".to_string()])?;
        }
        if let Some(value) = kwargs.get_item("remove_braces")? {
            options.remove_braces =
                defaultable_string_list(&value, "remove_braces", vec!["title".to_string()])?;
        }
        if let Some(value) = kwargs.get_item("wrap")? {
            options.wrap =
                defaultable_usize(&value, "wrap", CoreTidyOptions::default().with_wrap().wrap)?;
        }

        Ok(Self { inner: options })
    }

    fn __repr__(&self) -> String {
        format!(
            "TidyOptions(space={}, align={}, lowercase={}, escape={})",
            self.inner.space,
            option_usize(self.inner.align),
            self.inner.lowercase,
            self.inner.escape
        )
    }
}

#[pyclass(module = "refkit_core", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct TidyWarning {
    #[pyo3(get)]
    code: String,
    #[pyo3(get)]
    rule: Option<String>,
    #[pyo3(get)]
    message: String,
}

#[pymethods]
impl TidyWarning {
    fn __repr__(&self) -> String {
        format!(
            "TidyWarning(code={}, rule={}, message={})",
            quoted(&self.code),
            option_quoted(self.rule.as_deref()),
            quoted(&self.message)
        )
    }
}

#[pyclass(module = "refkit_core", frozen, skip_from_py_object)]
#[derive(Clone)]
pub struct TidyResult {
    #[pyo3(get)]
    bibtex: String,
    #[pyo3(get)]
    warnings: Vec<TidyWarning>,
    #[pyo3(get)]
    count: usize,
}

#[pymethods]
impl TidyResult {
    fn __repr__(&self) -> String {
        format!(
            "TidyResult({} entries, {} warnings)",
            self.count,
            self.warnings.len()
        )
    }
}

#[pyfunction]
#[pyo3(name = "tidy_bibtex", signature = (source, *, options = None))]
fn tidy_bibtex_py(
    py: Python<'_>,
    source: String,
    options: Option<PyRef<'_, TidyOptions>>,
) -> PyResult<TidyResult> {
    let options = options
        .as_ref()
        .map(|options| options.inner.clone())
        .unwrap_or_default();
    let result = py
        .detach(move || core_tidy_bibtex(&source, options))
        .map_err(|err| tidy_error_to_py(py, err))?;
    Ok(TidyResult::from_core(result))
}

impl TidyOptions {
    pub(crate) fn inner(&self) -> CoreTidyOptions {
        self.inner.clone()
    }
}

impl TidyResult {
    pub(crate) fn from_core(result: CoreTidyResult) -> Self {
        Self {
            bibtex: result.bibtex,
            warnings: result
                .warnings
                .into_iter()
                .map(TidyWarning::from_core)
                .collect(),
            count: result.count,
        }
    }
}

impl TidyWarning {
    fn from_core(warning: CoreTidyWarning) -> Self {
        match warning {
            CoreTidyWarning::MissingKey { message } => Self {
                code: "missing_key".to_string(),
                rule: None,
                message,
            },
            CoreTidyWarning::DuplicateEntry { rule, message } => Self {
                code: "duplicate_entry".to_string(),
                rule: Some(rule.as_str().to_string()),
                message,
            },
        }
    }
}

pub(crate) fn tidy_error_to_py(py: Python<'_>, err: CoreTidyError) -> PyErr {
    match err {
        CoreTidyError::Syntax {
            line,
            column,
            byte,
            character,
            message,
        } => {
            let pyerr = TidySyntaxError::new_err(message.clone());
            let value = pyerr.value(py);
            let _ = value.setattr("line", line);
            let _ = value.setattr("column", column);
            let _ = value.setattr("byte", byte);
            match character {
                Some(ch) => {
                    let _ = value.setattr("character", ch.to_string());
                }
                None => {
                    let _ = value.setattr("character", py.None());
                }
            }
            let _ = value.setattr("message", message);
            pyerr
        }
        CoreTidyError::Template(message) | CoreTidyError::Name(message) => {
            PyTidyError::new_err(message)
        }
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<TidyOptions>()?;
    module.add_class::<TidyWarning>()?;
    module.add_class::<TidyResult>()?;
    module.add("_tidy_option_names", TIDY_OPTION_NAMES.to_vec())?;
    module.add_function(wrap_pyfunction!(tidy_bibtex_py, module)?)?;
    Ok(())
}

const TIDY_OPTION_NAMES: &[&str] = &[
    "omit",
    "curly",
    "numeric",
    "months",
    "space",
    "tab",
    "align",
    "blank_lines",
    "sort",
    "duplicates",
    "merge",
    "strip_enclosing_braces",
    "drop_all_caps",
    "escape",
    "sort_fields",
    "strip_comments",
    "trailing_commas",
    "encode_urls",
    "tidy_comments",
    "remove_empty_fields",
    "remove_duplicate_fields",
    "generate_keys",
    "max_authors",
    "lowercase",
    "enclosing_braces",
    "remove_braces",
    "wrap",
];

fn bool_value(value: &Bound<'_, PyAny>, option: &str) -> PyResult<bool> {
    if value.is_none() {
        return Err(PyTypeError::new_err(format!(
            "{} must be a bool",
            quoted(option)
        )));
    }
    value
        .extract::<bool>()
        .map_err(|_| PyTypeError::new_err(format!("{} must be a bool", quoted(option))))
}

fn usize_value(value: &Bound<'_, PyAny>, option: &str) -> PyResult<usize> {
    if value.is_instance_of::<PyBool>() {
        return Err(PyTypeError::new_err(format!(
            "{} must be an integer",
            quoted(option)
        )));
    }
    value
        .extract::<usize>()
        .map_err(|_| PyTypeError::new_err(format!("{} must be an integer", quoted(option))))
}

fn optional_usize(value: &Bound<'_, PyAny>, option: &str) -> PyResult<Option<usize>> {
    if value.is_none() {
        Ok(None)
    } else {
        Ok(Some(usize_value(value, option)?))
    }
}

fn defaultable_usize(
    value: &Bound<'_, PyAny>,
    option: &str,
    default: Option<usize>,
) -> PyResult<Option<usize>> {
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(enabled) = value.extract::<bool>() {
        return Ok(if enabled { default } else { None });
    }
    Ok(Some(usize_value(value, option)?))
}

fn string_list(value: &Bound<'_, PyAny>, option: &str) -> PyResult<Vec<String>> {
    if value.is_none() {
        return Ok(Vec::new());
    }
    if value.extract::<String>().is_ok() {
        return Err(PyTypeError::new_err(format!(
            "{} must be an iterable of strings",
            quoted(option)
        )));
    }
    let iter = value.try_iter().map_err(|_| {
        PyTypeError::new_err(format!("{} must be an iterable of strings", quoted(option)))
    })?;
    iter.map(|item| item?.extract::<String>())
        .collect::<PyResult<Vec<_>>>()
}

fn defaultable_string_list(
    value: &Bound<'_, PyAny>,
    option: &str,
    default: Vec<String>,
) -> PyResult<Option<Vec<String>>> {
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(enabled) = value.extract::<bool>() {
        return Ok(if enabled { Some(default) } else { None });
    }
    Ok(Some(string_list(value, option)?))
}

fn duplicate_rules(value: &Bound<'_, PyAny>, option: &str) -> PyResult<Option<Vec<DuplicateRule>>> {
    if value.is_none() {
        return Ok(None);
    }
    let values = string_list(value, option)?;
    values
        .into_iter()
        .map(|value| match value.as_str() {
            "doi" => Ok(DuplicateRule::Doi),
            "key" => Ok(DuplicateRule::Key),
            "abstract" => Ok(DuplicateRule::Abstract),
            "citation" => Ok(DuplicateRule::Citation),
            _ => Err(PyValueError::new_err(format!(
                "unknown duplicate rule {}",
                quoted(&value)
            ))),
        })
        .collect::<PyResult<Vec<_>>>()
        .map(Some)
}

fn merge_strategy(value: &Bound<'_, PyAny>, option: &str) -> PyResult<Option<MergeStrategy>> {
    if value.is_none() {
        return Ok(None);
    }
    let value = value
        .extract::<String>()
        .map_err(|_| PyTypeError::new_err(format!("{} must be a string", quoted(option))))?;
    match value.as_str() {
        "first" => Ok(Some(MergeStrategy::First)),
        "last" => Ok(Some(MergeStrategy::Last)),
        "combine" => Ok(Some(MergeStrategy::Combine)),
        "overwrite" => Ok(Some(MergeStrategy::Overwrite)),
        _ => Err(PyValueError::new_err(format!(
            "unknown merge strategy {}",
            quoted(&value)
        ))),
    }
}

fn generate_keys(value: &Bound<'_, PyAny>, option: &str) -> PyResult<Option<String>> {
    if value.is_none() {
        return Ok(None);
    }
    if let Ok(enabled) = value.extract::<bool>() {
        return Ok(if enabled {
            CoreTidyOptions::default()
                .with_generate_keys()
                .generate_keys
        } else {
            None
        });
    }
    value
        .extract::<String>()
        .map(Some)
        .map_err(|_| PyTypeError::new_err(format!("{} must be a bool or string", quoted(option))))
}

fn default_sort_fields() -> Vec<String> {
    CoreTidyOptions::default()
        .with_sort()
        .sort
        .unwrap_or_default()
}

fn default_field_sort() -> Vec<String> {
    CoreTidyOptions::default()
        .with_sort_fields()
        .sort_fields
        .unwrap_or_default()
}

fn option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "None".to_string())
}
