//! Word features for the token tagger, hashed to `u64` with FNV-1a, which (unlike std's
//! hasher) is fixed forever, so the shipped `model.bin` keeps its meaning.

use crate::{Kind, Token};

/// The features of word `i`. `rule` is what the rules parser says about each word:
/// its token kind and whether the word starts that token.
pub(super) fn of(
    words: &[String],
    rule: &[Option<(Kind, bool)>],
    i: usize,
    prev: usize,
) -> Vec<u64> {
    let w = words[i].as_str();
    let at = |k: isize| {
        usize::try_from(i as isize + k)
            .ok()
            .and_then(|j| words.get(j))
            .map_or("<>", String::as_str)
    };
    let rule_at = |k: isize| {
        usize::try_from(i as isize + k)
            .ok()
            .and_then(|j| rule.get(j).copied())
            .flatten()
    };
    let suffix: String = w.chars().rev().take(3).collect();
    let feats: [(&str, String); 13] = [
        ("bias", String::new()),
        ("w", w.into()),
        ("shape", shape(w)),
        ("suf", suffix),
        ("w-1", at(-1).into()),
        ("w+1", at(1).into()),
        ("w-1w", format!("{} {w}", at(-1))),
        ("ww+1", format!("{w} {}", at(1))),
        ("shape+1", shape(at(1))),
        ("rule", super::encode(rule_at(0)).to_string()),
        ("rule-1", super::encode(rule_at(-1)).to_string()),
        ("rule+1", super::encode(rule_at(1)).to_string()),
        ("prev", prev.to_string()),
    ];
    feats.iter().map(hash).collect()
}

/// For each word, the rules token covering it: its kind and whether the word starts it.
pub(super) fn rule_labels(starts: &[usize], tokens: &[Token]) -> Vec<Option<(Kind, bool)>> {
    starts
        .iter()
        .map(|s| {
            tokens
                .iter()
                .find(|t| t.start <= *s && *s < t.end)
                .map(|t| (t.kind, t.start == *s))
        })
        .collect()
}

/// Letters as `a`, digits as `9`, runs collapsed: `1h30m` is `9a9a`.
fn shape(w: &str) -> String {
    let mut out = String::new();
    for c in w.chars() {
        let s = match c {
            c if c.is_alphabetic() => 'a',
            c if c.is_ascii_digit() => '9',
            c => c,
        };
        if !out.ends_with(s) {
            out.push(s);
        }
    }
    out
}

fn hash((name, value): &(&str, String)) -> u64 {
    let bytes = name.bytes().chain([0xff]).chain(value.bytes());
    bytes.fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}
