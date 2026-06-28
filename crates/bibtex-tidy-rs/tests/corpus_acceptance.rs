use std::fs;
use std::path::{Path, PathBuf};

use bibtex_tidy_rs::{TidyOptions, tidy};
use walkdir::WalkDir;

#[test]
fn upstream_corpus_round_trips_after_alphanumeric_normalization() {
    let mut failures = Vec::new();
    for path in corpus_files() {
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let input = String::from_utf8_lossy(&bytes);
        let options = TidyOptions {
            escape: false,
            remove_duplicate_fields: false,
            ..TidyOptions::default()
        };

        match tidy(&input, options) {
            Ok(result) => {
                let expected = alpha_num_only(&input);
                let actual = alpha_num_only(&result.bibtex);
                if actual != expected {
                    failures.push(format!(
                        "{}: normalized output mismatch\nexpected around: {}\nactual around: {}",
                        display_fixture_path(&path),
                        around_first_diff(&expected, &actual, &expected),
                        around_first_diff(&expected, &actual, &actual)
                    ));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", display_fixture_path(&path))),
        }
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn corpus_files() -> Vec<PathBuf> {
    let root = corpus_root();
    let mut files = WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|path| path.extension().is_some_and(|extension| extension == "bib"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/tidy/corpus")
}

fn alpha_num_only(value: &str) -> String {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();
    normalized
        .as_bytes()
        .chunks(50)
        .map(|chunk| std::str::from_utf8(chunk).expect("ASCII-only normalization"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn around_first_diff(expected: &str, actual: &str, source: &str) -> String {
    let index = expected
        .char_indices()
        .zip(actual.char_indices())
        .find_map(|((expected_index, expected_ch), (_, actual_ch))| {
            if expected_ch == actual_ch {
                None
            } else {
                Some(expected_index)
            }
        })
        .unwrap_or_else(|| expected.len().min(actual.len()));
    around(source, index)
}

fn around(value: &str, index: usize) -> String {
    let start = value[..index]
        .char_indices()
        .rev()
        .nth(120)
        .map_or(0, |(idx, _)| idx);
    let end = value[index..]
        .char_indices()
        .nth(120)
        .map_or(value.len(), |(idx, _)| index + idx);
    format!("{:?}", &value[start..end])
}

fn display_fixture_path(path: &Path) -> String {
    path.strip_prefix(corpus_root())
        .unwrap_or(path)
        .display()
        .to_string()
}
