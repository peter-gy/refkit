pub use refkit_core::{
    DuplicateRule, MergeStrategy, TidyError, TidyOptions, TidyResult, TidyWarning,
};

pub struct BibtexTidy {
    options: TidyOptions,
}

impl BibtexTidy {
    pub fn new(options: TidyOptions) -> Self {
        Self { options }
    }

    pub fn tidy(&self, input: &str) -> Result<TidyResult, TidyError> {
        tidy(input, self.options.clone())
    }
}

pub fn tidy(input: &str, options: TidyOptions) -> Result<TidyResult, TidyError> {
    refkit_core::tidy_bibtex(input, options)
}
