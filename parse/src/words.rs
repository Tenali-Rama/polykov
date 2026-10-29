//! Splits an input line into words with byte offsets, keeping `"quoted text"` out of reach.

/// A whitespace-separated word, `start..end` in bytes. A `quoted` word overlaps a quoted span
/// and is never parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Word {
    pub start: usize,
    pub end: usize,
    pub quoted: bool,
}

/// The input's words, and the byte offsets of the paired `"` characters, which the title
/// drops. Quotes pair up in order; a last unpaired `"` is plain text. Whitespace inside a
/// quoted span doesn't split words.
pub(crate) fn split(input: &str) -> (Vec<Word>, Vec<usize>) {
    let marks: Vec<usize> = input.match_indices('"').map(|(i, _)| i).collect();
    let spans: Vec<(usize, usize)> = marks
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&[a, b]| (a, b))
        .collect();
    let in_span = |i: usize| spans.iter().any(|&(a, b)| a <= i && i <= b);
    let mut words = Vec::new();
    let mut start = None;
    for (i, c) in input.char_indices() {
        if c.is_whitespace() && !in_span(i) {
            if let Some(s) = start.take() {
                words.push(word(s, i, &spans));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        words.push(word(s, input.len(), &spans));
    }
    let quotes = spans.iter().flat_map(|&(a, b)| [a, b]).collect();
    (words, quotes)
}

fn word(start: usize, end: usize, spans: &[(usize, usize)]) -> Word {
    let quoted = spans.iter().any(|&(a, b)| a < end && start <= b);
    Word { start, end, quoted }
}

/// A run of unquoted words that a matcher reads from its first word on.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Run<'a> {
    pub input: &'a str,
    pub words: &'a [Word],
}

impl<'a> Run<'a> {
    /// The `k`-th word as written.
    pub fn raw(&self, k: usize) -> Option<&'a str> {
        self.words.get(k).map(|w| &self.input[w.start..w.end])
    }

    /// The `k`-th word, lowercased.
    pub fn low(&self, k: usize) -> Option<String> {
        self.raw(k).map(str::to_lowercase)
    }

    /// The run without its first `k` words.
    pub fn skip(&self, k: usize) -> Run<'a> {
        Run {
            input: self.input,
            words: self.words.get(k..).unwrap_or(&[]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(input: &str) -> Vec<(&str, bool)> {
        let (words, _) = split(input);
        words
            .iter()
            .map(|w| (&input[w.start..w.end], w.quoted))
            .collect()
    }

    #[test]
    fn splits_on_any_whitespace_with_byte_offsets() {
        let (words, quotes) = split("  call\tmé  now ");
        let spans: Vec<_> = words.iter().map(|w| (w.start, w.end)).collect();
        assert_eq!(spans, [(2, 6), (7, 10), (12, 15)]);
        assert!(quotes.is_empty());
    }

    #[test]
    fn quoted_span_is_one_protected_word() {
        assert_eq!(
            texts(r#"read "next week" now"#),
            [("read", false), (r#""next week""#, true), ("now", false)]
        );
        let (_, quotes) = split(r#"read "next week" now"#);
        assert_eq!(quotes, [5, 15]);
    }

    #[test]
    fn unpaired_quote_is_plain_text() {
        assert_eq!(texts(r#"say "hi"#), [("say", false), (r#""hi"#, false)]);
        assert!(split(r#"say "hi"#).1.is_empty());
    }
}
