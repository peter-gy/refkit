#[derive(Default)]
struct BracePairs {
    pending: Vec<usize>,
    matched: Vec<(usize, usize)>,
}

impl BracePairs {
    fn accept(&mut self, offset: usize, character: char) {
        match character {
            '{' => self.pending.push(offset),
            '}' => {
                if let Some(open) = self.pending.pop() {
                    self.matched.push((open, offset + 1));
                }
            }
            _ => {}
        }
    }

    fn into_pairs(self) -> Vec<(usize, usize)> {
        self.matched
    }
}

/// Each opener is an independent scan start: nested regions skip braces opened
/// after them, never an enclosing brace from an earlier malformed block. Pair
/// tables contain source byte positions. Root quotes and comments remain the
/// caller's responsibility.
#[derive(Default)]
pub(super) struct BoundaryIndex {
    escaped: Vec<(usize, usize)>,
    literal: Option<Vec<(usize, usize)>>,
    quotes: Vec<(usize, usize)>,
    openers: Vec<usize>,
    curly_key_ends: Vec<usize>,
    paren_key_ends: Vec<usize>,
}

impl BoundaryIndex {
    pub(super) fn new(source: &str) -> Self {
        if !source.contains('@') {
            return Self::default();
        }
        let mut escaped_braces = BracePairs::default();
        let mut literal_braces = source.contains('\\').then(BracePairs::default);
        let mut openers = Vec::new();
        let mut curly_key_ends = Vec::new();
        let mut paren_key_ends = Vec::new();
        let mut escaped = false;
        for (offset, character) in source.char_indices() {
            match character {
                '{' | '(' => openers.push(offset),
                ',' | '=' => {
                    curly_key_ends.push(offset);
                    paren_key_ends.push(offset);
                }
                '}' => curly_key_ends.push(offset),
                ')' => paren_key_ends.push(offset),
                _ => {}
            }
            if !escaped {
                escaped_braces.accept(offset, character);
            }
            if let Some(literal) = &mut literal_braces {
                literal.accept(offset, character);
            }
            escaped = !escaped && character == '\\';
        }
        let escaped_braces = escaped_braces.into_pairs();
        let literal_braces = literal_braces.map(BracePairs::into_pairs);
        let quotes = quote_ends(source, &escaped_braces);
        let escaped = parenthesis_ends(source, escaped_braces, true);
        let literal = literal_braces.map(|braces| parenthesis_ends(source, braces, false));
        Self {
            escaped,
            literal,
            quotes,
            openers,
            curly_key_ends,
            paren_key_ends,
        }
    }

    pub(super) fn next_opener(&self, start: usize) -> Option<usize> {
        next_position(&self.openers, start)
    }

    pub(super) fn key_end(&self, start: usize, closer: char) -> Option<usize> {
        let positions = if closer == '}' {
            &self.curly_key_ends
        } else {
            &self.paren_key_ends
        };
        next_position(positions, start)
    }

    pub(super) fn delimiter_end(&self, start: usize, escape_aware: bool) -> Option<usize> {
        let pairs = if escape_aware {
            &self.escaped
        } else {
            self.literal.as_ref().unwrap_or(&self.escaped)
        };
        paired_end(pairs, start)
    }

    pub(super) fn quote_end(&self, start: usize) -> Option<usize> {
        paired_end(&self.quotes, start)
    }
}

// A brace can restore a successor already consumed by an opener inside it.
// Nodes are appended once per closing parenthesis, without copying chains.
struct Successor {
    end: usize,
    next: Option<usize>,
}

fn parenthesis_ends(
    source: &str,
    mut braces: Vec<(usize, usize)>,
    escape_aware: bool,
) -> Vec<(usize, usize)> {
    if !source.contains('(') {
        braces.sort_unstable_by_key(|(open, _)| *open);
        return braces;
    }
    let mut closings = braces.iter().rev().peekable();
    let mut restorations = Vec::new();
    let mut successors: Vec<Successor> = Vec::new();
    let mut pairs = Vec::new();
    let mut head = None;
    for (offset, character) in source.char_indices().rev() {
        if !matches!(character, '(' | ')' | '{' | '}')
            || (escape_aware && is_escaped(source, offset))
        {
            continue;
        }
        match character {
            ')' => {
                successors.push(Successor {
                    end: offset + 1,
                    next: head,
                });
                head = Some(successors.len() - 1);
            }
            '(' => {
                if let Some(successor) = head.and_then(|index| successors.get(index)) {
                    pairs.push((offset, successor.end));
                    head = successor.next;
                }
            }
            '}' if closings.peek().is_some_and(|(_, end)| *end == offset + 1) => {
                restorations.push(head);
                closings.next();
            }
            '{' => head = restorations.pop().flatten(),
            _ => {}
        }
    }
    pairs.extend(braces);
    pairs.sort_unstable_by_key(|(open, _)| *open);
    pairs
}

fn next_position(positions: &[usize], start: usize) -> Option<usize> {
    positions
        .get(positions.partition_point(|position| *position < start))
        .copied()
}

fn paired_end(pairs: &[(usize, usize)], start: usize) -> Option<usize> {
    let index = pairs.binary_search_by_key(&start, |(open, _)| *open).ok()?;
    pairs.get(index).map(|(_, end)| *end)
}

fn is_escaped(source: &str, offset: usize) -> bool {
    source[..offset]
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'\\')
        .count()
        % 2
        == 1
}

fn quote_ends(source: &str, braces: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if !source.contains('"') {
        return Vec::new();
    }
    let mut closings = braces.iter().rev().peekable();
    let mut restorations = Vec::new();
    let mut pairs = Vec::new();
    let mut next_quote = None;
    for (offset, character) in source.char_indices().rev() {
        if !matches!(character, '"' | '{' | '}') {
            continue;
        }
        let escaped = is_escaped(source, offset);
        if character == '"' {
            if let Some(end) = next_quote {
                pairs.push((offset, end));
            }
            if !escaped {
                next_quote = Some(offset + 1);
            }
            continue;
        }
        if escaped {
            continue;
        }
        match character {
            '}' if closings.peek().is_some_and(|(_, end)| *end == offset + 1) => {
                restorations.push(next_quote);
                closings.next();
            }
            '{' => next_quote = restorations.pop().flatten(),
            _ => {}
        }
    }
    pairs.reverse();
    pairs
}
