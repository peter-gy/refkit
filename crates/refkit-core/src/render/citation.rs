use std::str::FromStr;

use hayagriva::citationberg::LocaleCode;
use hayagriva::citationberg::taxonomy::Locator;
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest as EngineRequest,
    LocatorPayload, Rendered, SpecificLocator,
};

use crate::{CitationRequest, Cite, DocumentError, Library, PreparedStyle};

use super::bundled_locales;

pub(crate) fn process_citations(
    library: &Library,
    style: &PreparedStyle,
    locale: Option<&str>,
    requests: &[CitationRequest],
) -> Result<Rendered, DocumentError> {
    let locales = bundled_locales();
    let locale = locale.map(|code| LocaleCode(code.to_string()));
    let mut driver = BibliographyDriver::new();

    for request in requests {
        if request.items.is_empty() {
            return Err(DocumentError::EmptyCitation);
        }
        if request
            .note_number
            .is_some_and(|number| number == 0 || number > u32::MAX as usize)
        {
            return Err(DocumentError::InvalidNoteNumber);
        }
        let items = request
            .items
            .iter()
            .map(|cite| {
                let entry = library
                    .inner()
                    .get(&cite.key)
                    .ok_or_else(|| DocumentError::MissingReference(cite.key.clone()))?;
                let locator = cite
                    .locator
                    .as_deref()
                    .map(|value| {
                        let label = cite.label.as_deref().unwrap_or("page");
                        let label = Locator::from_str(label)
                            .map_err(|_| DocumentError::UnknownLocatorLabel(label.to_string()))?;
                        Ok(SpecificLocator(label, LocatorPayload::Str(value)))
                    })
                    .transpose()?;
                Ok(CitationItem::with_locator(entry, locator))
            })
            .collect::<Result<Vec<_>, DocumentError>>()?;
        driver.citation(EngineRequest::new(
            items,
            style.inner.as_ref(),
            locale.clone(),
            locales,
            request.note_number,
        ));
    }

    let rendered = driver.finish(BibliographyRequest::new(
        style.inner.as_ref(),
        locale,
        locales,
    ));
    if rendered.citations.len() != requests.len() {
        return Err(DocumentError::Render(
            "citation renderer returned an unexpected citation count".to_string(),
        ));
    }
    Ok(rendered)
}

pub(crate) fn request_for_keys(keys: &[&str]) -> CitationRequest {
    CitationRequest::new(
        keys.iter()
            .map(|key| Cite::new((*key).to_string(), None, None))
            .collect(),
        None,
    )
}

pub(crate) fn full_bibliography_requests(library: &Library) -> Vec<CitationRequest> {
    library
        .keys()
        .iter()
        .map(|key| request_for_keys(&[key]))
        .collect()
}
