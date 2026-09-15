use crate::raw::{RawEntryData, RawEntryId, RawFieldData, RawFieldId};

pub(super) struct EntryView<'a> {
    pub id: RawEntryId,
    pub key: &'a str,
    pub kind: &'a str,
    fields: &'a [RawFieldData],
}

impl<'a> EntryView<'a> {
    pub(super) fn new(index: usize, entry: &'a RawEntryData) -> Self {
        Self {
            id: RawEntryId::from_index(index),
            key: &entry.key,
            kind: &entry.kind,
            fields: &entry.field_blocks,
        }
    }

    pub(super) fn fields(&self) -> impl Iterator<Item = (RawFieldId, &'a RawFieldData)> {
        self.fields
            .iter()
            .enumerate()
            .map(|(index, field)| (RawFieldId::from_index(index), field))
    }

    pub(super) fn field(&self, name: &str) -> Option<&'a str> {
        self.fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(name))
            .map(|field| field.value.as_str())
    }
}
