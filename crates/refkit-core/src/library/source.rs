use std::ops::Range;

pub(super) struct RecoverySource {
    pub text: String,
    origins: Option<Vec<usize>>,
}

impl RecoverySource {
    pub fn new(text: String) -> Self {
        Self {
            text,
            origins: None,
        }
    }

    pub fn original_span(&self, span: Range<usize>) -> Range<usize> {
        match &self.origins {
            Some(origins) => {
                origins[span.start.min(self.text.len())]..origins[span.end.min(self.text.len())]
            }
            None => span,
        }
    }

    pub fn literalize(&mut self, span: Range<usize>) {
        let origins = self
            .origins
            .get_or_insert_with(|| (0..=self.text.len()).collect());
        let start = origins[span.start];
        let end = origins[span.end];
        let replacement_origins = std::iter::once(start)
            .chain(origins[span.clone()].iter().copied())
            .chain(std::iter::once(end))
            .collect::<Vec<_>>();
        let replacement = format!("{{{}}}", &self.text[span.clone()]);
        self.text.replace_range(span.clone(), &replacement);
        origins.splice(span, replacement_origins);
    }

    pub fn replace(&mut self, span: Range<usize>, replacement: &str) {
        let original = self.original_span(span.clone());
        let origins = self
            .origins
            .get_or_insert_with(|| (0..=self.text.len()).collect());
        self.text.replace_range(span.clone(), replacement);
        origins.splice(span, std::iter::repeat_n(original.start, replacement.len()));
    }
}
