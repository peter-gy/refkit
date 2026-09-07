use polars::prelude::*;

use super::EntriesKwargs;

pub(super) fn keys_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(DataType::String)),
    ))
}

pub(super) fn boolean_output(input_fields: &[Field]) -> PolarsResult<Field> {
    fixed_output(input_fields, DataType::Boolean)
}

pub(super) fn string_output(input_fields: &[Field]) -> PolarsResult<Field> {
    fixed_output(input_fields, DataType::String)
}

pub(super) fn uint32_output(input_fields: &[Field]) -> PolarsResult<Field> {
    fixed_output(input_fields, DataType::UInt32)
}

pub(super) fn entries_output(input_fields: &[Field], kwargs: EntriesKwargs) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    let field_names = kwargs.fields.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(entry_struct_dtype(&field_names))),
    ))
}

pub(super) fn rendered_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, rendered_struct_dtype()))
}

pub(super) fn rendered_list_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(rendered_struct_dtype())),
    ))
}

pub(super) fn parse_report_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, parse_report_struct_dtype()))
}

pub(super) fn tidy_report_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, tidy_report_struct_dtype()))
}

pub(super) fn entry_struct_dtype(field_names: &[&str]) -> DataType {
    DataType::Struct(
        field_names
            .iter()
            .map(|field| Field::new((*field).into(), DataType::String))
            .collect(),
    )
}

pub(super) fn rendered_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("text".into(), DataType::String),
        Field::new("html".into(), DataType::String),
    ])
}

fn parse_report_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("ok".into(), DataType::Boolean),
        Field::new("entry_count".into(), DataType::UInt32),
        Field::new("keys".into(), DataType::List(Box::new(DataType::String))),
        Field::new(
            "diagnostics".into(),
            DataType::List(Box::new(diagnostic_struct_dtype())),
        ),
    ])
}

pub(super) fn tidy_warning_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("code".into(), DataType::String),
        Field::new("rule".into(), DataType::String),
        Field::new("message".into(), DataType::String),
    ])
}

fn tidy_report_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("ok".into(), DataType::Boolean),
        Field::new("bibtex".into(), DataType::String),
        Field::new("count".into(), DataType::UInt32),
        Field::new(
            "warnings".into(),
            DataType::List(Box::new(tidy_warning_struct_dtype())),
        ),
        Field::new(
            "renames".into(),
            DataType::List(Box::new(tidy_rename_struct_dtype())),
        ),
        Field::new("error".into(), DataType::String),
    ])
}

fn fixed_output(input_fields: &[Field], dtype: DataType) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, dtype))
}

pub(super) fn diagnostic_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("code".into(), DataType::String),
        Field::new("severity".into(), DataType::String),
        Field::new("action".into(), DataType::String),
        Field::new(
            "span".into(),
            DataType::Struct(vec![
                Field::new("start".into(), DataType::UInt64),
                Field::new("end".into(), DataType::UInt64),
            ]),
        ),
        Field::new("entry".into(), DataType::String),
        Field::new("field".into(), DataType::String),
        Field::new("message".into(), DataType::String),
    ])
}

pub(super) fn diagnostics_output(input_fields: &[Field]) -> PolarsResult<Field> {
    fixed_output(
        input_fields,
        DataType::List(Box::new(diagnostic_struct_dtype())),
    )
}

pub(super) fn tidy_rename_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("entry_id".into(), DataType::UInt64),
        Field::new("old_key".into(), DataType::String),
        Field::new("new_key".into(), DataType::String),
    ])
}

pub(super) fn render_report_output(input_fields: &[Field]) -> PolarsResult<Field> {
    validate_key_lists(input_fields)?;
    fixed_output(
        input_fields,
        DataType::Struct(vec![
            Field::new("ok".into(), DataType::Boolean),
            Field::new(
                "citations".into(),
                DataType::List(Box::new(rendered_struct_dtype())),
            ),
            Field::new(
                "diagnostics".into(),
                DataType::List(Box::new(diagnostic_struct_dtype())),
            ),
            Field::new("error_code".into(), DataType::String),
            Field::new("error".into(), DataType::String),
        ]),
    )
}

pub(super) fn with_struct_validity(
    chunked: StructChunked,
    valid: impl Iterator<Item = bool>,
) -> PolarsResult<Series> {
    use pyo3_polars::export::polars_arrow::bitmap::Bitmap;
    let valid = Bitmap::from_iter(valid);
    Ok(chunked
        .rechunk()
        .into_owned()
        .with_outer_validity(Some(valid))
        .into_series())
}

fn validate_key_lists(input_fields: &[Field]) -> PolarsResult<()> {
    if input_fields[1].dtype != DataType::List(Box::new(DataType::String)) {
        polars_bail!(InvalidOperation: "citation keys must have dtype List[String], got {}", input_fields[1].dtype);
    }
    Ok(())
}

pub(super) fn each_string_output(input_fields: &[Field]) -> PolarsResult<Field> {
    validate_key_lists(input_fields)?;
    keys_output(input_fields)
}

pub(super) fn each_rendered_output(input_fields: &[Field]) -> PolarsResult<Field> {
    validate_key_lists(input_fields)?;
    rendered_list_output(input_fields)
}

pub(super) fn group_string_output(input_fields: &[Field]) -> PolarsResult<Field> {
    validate_key_lists(input_fields)?;
    string_output(input_fields)
}

pub(super) fn group_rendered_output(input_fields: &[Field]) -> PolarsResult<Field> {
    validate_key_lists(input_fields)?;
    rendered_output(input_fields)
}
