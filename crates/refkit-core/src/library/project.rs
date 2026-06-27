use crate::quoted;

use super::{EntryRecord, ProjectField};

pub fn parse_project_field(field: &str) -> Result<ProjectField, String> {
    match field {
        "key" => Ok(ProjectField::Key),
        "entry_type" => Ok(ProjectField::EntryType),
        "type" => Ok(ProjectField::Type),
        "title" => Ok(ProjectField::Title),
        "doi" => Ok(ProjectField::Doi),
        "volume" => Ok(ProjectField::Volume),
        _ => Err(format!("unsupported projection field {}", quoted(field))),
    }
}

pub(super) fn project_record(record: &EntryRecord, fields: &[ProjectField]) -> Vec<Option<String>> {
    fields
        .iter()
        .map(|field| match field {
            ProjectField::Key => Some(record.key.clone()),
            ProjectField::EntryType | ProjectField::Type => Some(record.entry_type.clone()),
            ProjectField::Title => record.title.clone(),
            ProjectField::Doi => record.doi.clone(),
            ProjectField::Volume => record.volume.clone(),
        })
        .collect()
}
