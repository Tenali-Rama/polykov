//! Matching one query word against one indexed text, and the bracketed snippet. Texts and
//! words are compared as case-folded `char`s, one folded char per original char, so match
//! positions index the original text too.

/// Shortest word the trigram index can look up.
pub const MIN_INDEXED: usize = 3;
/// Shortest word that may match with one typo. Below this, one of its halves would be too
/// short for the index (ADR 0014).
pub const MIN_FUZZY: usize = 2 * MIN_INDEXED;
/// Longest snippet, in characters, before the ellipses.
const SNIPPET_CHARS: usize = 80;
/// Characters of context kept before the first match in a long snippet.
const SNIPPET_LEAD: usize = 20;

/// How well a word matched; higher is better.
pub mod quality {
    pub const TYPO: f64 = 1.0;
    pub const SUBSTRING: f64 = 2.0;
    pub const PREFIX: f64 = 3.0;
    pub const WORD: f64 = 4.0;
}

/// Where a word matched, as a char range of the text, and how well.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    pub start: usize,
    pub end: usize,
    pub quality: f64,
}

/// `c` case-folded to a single char.
pub fn fold(c: char) -> char {
    if c.is_ascii() {
        c.to_ascii_lowercase()
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// The words of a query: its runs of letters and digits, case-folded. Everything else,
/// FTS syntax included, only separates words.
pub fn words(query: &str) -> Vec<Vec<char>> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.chars().map(fold).collect())
        .collect()
}

/// The best match of `word` in the folded `text`: an exact one if any (a whole word beats a
/// word prefix beats a substring), else, if `typos` is set, one with a single typo for words
/// of [`MIN_FUZZY`]+ characters.
pub fn find(word: &[char], text: &[char], typos: bool) -> Option<Found> {
    let mut best: Option<Found> = None;
    if word.len() <= text.len() {
        for start in 0..=text.len() - word.len() {
            let end = start + word.len();
            if text[start..end] != *word {
                continue;
            }
            let quality = match (boundary(text, start), boundary(text, end)) {
                (true, true) => quality::WORD,
                (true, false) => quality::PREFIX,
                _ => quality::SUBSTRING,
            };
            if best.is_none_or(|b| quality > b.quality) {
                best = Some(Found {
                    start,
                    end,
                    quality,
                });
            }
        }
    }
    if best.is_none() && typos && word.len() >= MIN_FUZZY {
        best = typo(word, text);
    }
    best
}

/// The quality of the best match of `word` at the start of a word of `text` (whole word or
/// prefix), as [`find`] would rate it, without folding all of `text` first: the per-row
/// check of queries the index can't narrow, which must stay cheap.
pub fn word_start(word: &[char], text: &str) -> Option<f64> {
    let mut best = None;
    let mut after_word = false;
    for (i, c) in text.char_indices() {
        let alphanumeric = c.is_alphanumeric();
        if alphanumeric && !after_word {
            let mut rest = text[i..].chars();
            if word.iter().all(|&w| rest.next().map(fold) == Some(w)) {
                if !rest.next().is_some_and(char::is_alphanumeric) {
                    return Some(quality::WORD);
                }
                best = Some(quality::PREFIX);
            }
        }
        after_word = alphanumeric;
    }
    best
}

/// Whether a word boundary lies before char `i` of `text` (`i == len` included).
fn boundary(text: &[char], i: usize) -> bool {
    i == 0 || i == text.len() || !text[i - 1].is_alphanumeric() || !text[i].is_alphanumeric()
}

/// The first substring of `text` within edit distance 1 of `word` (Sellers' algorithm,
/// tracking where each alignment starts).
fn typo(word: &[char], text: &[char]) -> Option<Found> {
    let n = text.len();
    let mut cost: Vec<usize> = vec![0; n + 1];
    let mut from: Vec<usize> = (0..=n).collect();
    for (i, &w) in word.iter().enumerate() {
        let mut next = vec![i + 1; n + 1];
        let mut next_from = vec![0; n + 1];
        for j in 1..=n {
            let options = [
                (cost[j - 1] + usize::from(text[j - 1] != w), from[j - 1]),
                (cost[j] + 1, from[j]),
                (next[j - 1] + 1, next_from[j - 1]),
            ];
            let (c, f) = options
                .into_iter()
                .min_by_key(|&(c, _)| c)
                .unwrap_or((usize::MAX, 0));
            next[j] = c;
            next_from[j] = f;
        }
        cost = next;
        from = next_from;
    }
    (1..=n).find(|&j| cost[j] <= 1).map(|end| Found {
        start: from[end],
        end,
        quality: quality::TYPO,
    })
}

/// `text` with each of `spans` (char ranges) wrapped in `[` `]`, cut to about
/// [`SNIPPET_CHARS`] around the first span, on one line.
pub fn snippet(text: &str, spans: &[(usize, usize)]) -> String {
    let chars: Vec<char> = text
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    let mut spans = spans.to_vec();
    spans.sort_unstable();
    let first = spans.first().map_or(0, |s| s.0);
    let (from, to) = if chars.len() <= SNIPPET_CHARS {
        (0, chars.len())
    } else {
        let from = first.saturating_sub(SNIPPET_LEAD);
        (from, (from + SNIPPET_CHARS).min(chars.len()))
    };
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    let mut at = from;
    for (start, end) in spans {
        let (start, end) = (start.max(at), end.min(to));
        if start >= end {
            continue;
        }
        out.extend(&chars[at..start]);
        out.push('[');
        out.extend(&chars[start..end]);
        out.push(']');
        at = end;
    }
    out.extend(&chars[at..to]);
    if to < chars.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folded(s: &str) -> Vec<char> {
        s.chars().map(fold).collect()
    }

    fn found(word: &str, text: &str) -> Option<Found> {
        find(&folded(word), &folded(text), true)
    }

    #[test]
    fn words_drop_syntax_and_fold_case() {
        let w = words(r#"  "Meet" AND* NEAR(x  "#);
        assert_eq!(
            w,
            vec![folded("meet"), folded("and"), folded("near"), folded("x")]
        );
        assert!(words(r#"" * ( )"#).is_empty());
    }

    #[test]
    fn exact_matches_rank_word_over_prefix_over_substring() {
        assert_eq!(
            found("notes", "Meeting notes").unwrap().quality,
            quality::WORD
        );
        let prefix = found("meet", "Meeting notes").unwrap();
        assert_eq!(
            (prefix.start, prefix.end, prefix.quality),
            (0, 4, quality::PREFIX)
        );
        assert_eq!(
            found("ocer", "Groceries").unwrap().quality,
            quality::SUBSTRING
        );
        assert_eq!(
            found("ting", "a meeting").unwrap().quality,
            quality::SUBSTRING
        );
        assert_eq!(found("ab", "xab ab").unwrap().start, 4);
    }

    #[test]
    fn one_typo_matches_long_words_only() {
        let hit = found("groceris", "Buy groceries").unwrap();
        assert_eq!(hit.quality, quality::TYPO);
        assert_eq!(hit.start, 4);
        assert!(found("grocxriz", "Buy groceries").is_none());
        assert!(found("grocs", "groceries").is_none());
        assert!(found("meeting", "").is_none());
        assert_eq!(found("meetimg", "meeting").unwrap().quality, quality::TYPO);
        assert!(find(&folded("groceris"), &folded("groceries"), false).is_none());
    }

    #[test]
    fn word_start_rates_like_find_but_ignores_inner_matches() {
        let rate = |word: &str, text: &str| word_start(&folded(word), text);
        assert_eq!(rate("me", "Team Me"), Some(quality::WORD));
        assert_eq!(rate("me", "Team meeting"), Some(quality::PREFIX));
        assert_eq!(rate("me", "(Meet)"), Some(quality::PREFIX));
        assert_eq!(rate("é", "Été"), Some(quality::PREFIX));
        assert_eq!(rate("me", "Some time"), None);
        assert_eq!(rate("me", "m"), None);
        for (word, text) in [("te", "Team meeting"), ("a", "work/a b"), ("ab", "ab")] {
            let found = find(&folded(word), &folded(text), false).map(|f| f.quality);
            assert_eq!(rate(word, text), found, "{word} in {text}");
        }
    }

    #[test]
    fn snippet_brackets_spans_and_trims_long_text() {
        assert_eq!(
            snippet("Meeting notes", &[(8, 13), (0, 4)]),
            "[Meet]ing [notes]"
        );
        let long = format!("{}needle{}", "a".repeat(100), " b".repeat(100));
        let s = snippet(&long, &[(100, 106)]);
        assert!(s.starts_with('…') && s.ends_with('…'));
        assert!(s.contains("[needle]"));
        assert_eq!(snippet("line\nbreak", &[]), "line break");
    }
}
