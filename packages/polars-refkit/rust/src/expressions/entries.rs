use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use refkit_core::{EntryField, EntryRecord};

use super::EntriesKwargs;
use super::broadcast::{compute_error, parse_value_library_source};
use super::dtypes::{entries_output, entry_struct_dtype};

#[polars_expr(output_type_func_with_kwargs=entries_output)]
fn entries(inputs: &[Series], kwargs: EntriesKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let fields = parse_project_fields(&kwargs.fields).map_err(compute_error)?;
    let field_names = kwargs.fields.iter().map(String::as_str).collect::<Vec<_>>();
    let mut builder = AnonymousOwnedListBuilder::new(
        "entries".into(),
        bibtex.len(),
        Some(entry_struct_dtype(&field_names)),
    );

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        match parse_value_library_source(source, kwargs.recovery.policy()) {
            Ok(library) => {
                let entries =
                    entry_records_to_struct_series(library.records(), &fields, &field_names)?;
                builder.append_series(&entries)?;
            }
            Err(_) => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

fn parse_project_fields(fields: &[String]) -> Result<Vec<EntryField>, String> {
    fields
        .iter()
        .map(|field| {
            field
                .parse::<EntryField>()
                .map_err(|error| error.to_string())
        })
        .collect()
}

fn entry_records_to_struct_series(
    records: &[EntryRecord],
    fields: &[EntryField],
    field_names: &[&str],
) -> PolarsResult<Series> {
    let fields = field_names
        .iter()
        .enumerate()
        .map(|(field_index, field_name)| {
            StringChunked::from_iter_options(
                (*field_name).into(),
                records
                    .iter()
                    .map(move |record| record.field(fields[field_index])),
            )
            .into_series()
        })
        .collect::<Vec<_>>();
    StructChunked::from_series("entry".into(), records.len(), fields.iter())
        .map(|entries| entries.into_series())
}
