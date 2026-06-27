use polars::prelude::*;

use super::EntriesKwargs;

pub(super) fn keys_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(DataType::String)),
    ))
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
            DataType::List(Box::new(DataType::String)),
        ),
    ])
}
