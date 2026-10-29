//! Single-word marks: `#tag`, `+project`, `@area`, `p1`–`p4` and durations.

/// The longest tag the server accepts (`tags::MAX_TAG_CHARS`).
const MAX_TAG_CHARS: usize = 100;
/// The longest estimate the server accepts, in minutes (one week).
const MAX_ESTIMATE_MIN: u32 = 10080;

/// `#tag` or `#a/b`, normalized like the server's tag rules: lowercase, non-empty
/// `/`-separated segments without `#` or `,`. The name must contain a letter, so `#1` is text.
pub(super) fn tag(w: &str) -> Option<String> {
    let t = sigil_name(w, '#')?.to_lowercase();
    let ok = t.chars().count() <= MAX_TAG_CHARS
        && t.split('/')
            .all(|s| !s.is_empty() && !s.contains(['#', ',']));
    ok.then_some(t)
}

/// `+name` (projects) or `@name` (areas): `Some(resolved)` if `w` is such a mark, where
/// `resolved` is the first of `known` that matches ignoring case, with `-`/`_` read as spaces.
pub(super) fn named(w: &str, sigil: char, known: &[String]) -> Option<Option<String>> {
    let want = fold(sigil_name(w, sigil)?);
    Some(known.iter().find(|k| fold(k) == want).cloned())
}

/// `p1`–`p4`, either case.
pub(super) fn priority(w: &str) -> Option<u8> {
    match w.to_lowercase().as_str() {
        "p1" => Some(1),
        "p2" => Some(2),
        "p3" => Some(3),
        "p4" => Some(4),
        _ => None,
    }
}

/// `30m`, `45min`, `2h`, `1h30m`, `1.5h` as whole minutes, 1–10080.
pub(super) fn duration(w: &str) -> Option<u32> {
    let w = w.to_lowercase();
    let minutes = |s: &str| {
        s.strip_suffix("min")
            .or_else(|| s.strip_suffix('m'))
            .and_then(digits)
    };
    let total = if let Some((h, rest)) = w.split_once('h') {
        let plain = h.chars().all(|c| c.is_ascii_digit() || c == '.');
        let hours: f64 = plain.then(|| h.parse().ok()).flatten()?;
        let extra = if rest.is_empty() { 0 } else { minutes(rest)? };
        (hours * 60.0).round() + f64::from(extra)
    } else {
        f64::from(minutes(&w)?)
    };
    let in_range = (1.0..=f64::from(MAX_ESTIMATE_MIN)).contains(&total);
    in_range.then_some(total as u32)
}

/// The rest of `w` after `sigil`, if it contains a letter.
fn sigil_name(w: &str, sigil: char) -> Option<&str> {
    w.strip_prefix(sigil)
        .filter(|name| name.chars().any(char::is_alphabetic))
}

/// Lowercase, `-`/`_` as spaces, whitespace collapsed.
fn fold(name: &str) -> String {
    let spaced = name.to_lowercase().replace(['-', '_'], " ");
    spaced.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn digits(s: &str) -> Option<u32> {
    let ok = !s.is_empty() && s.len() <= 5 && s.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| s.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_flat_and_hierarchical() {
        assert_eq!(tag("#family"), Some("family".into()));
        assert_eq!(tag("#Work/ClientA"), Some("work/clienta".into()));
        assert_eq!(tag("#a/b/c"), Some("a/b/c".into()));
        for w in [
            "#",
            "#1",
            "#a//b",
            "#a/",
            "#a,b",
            "##a",
            "family",
            &format!("#{}", "a".repeat(101)),
        ] {
            assert_eq!(tag(w), None, "{w}");
        }
    }

    #[test]
    fn projects_and_areas_match_known_names() {
        let known = vec!["Home Reno".to_string(), "Work".to_string()];
        assert_eq!(
            named("+home-reno", '+', &known),
            Some(Some("Home Reno".into()))
        );
        assert_eq!(
            named("+HOME_RENO", '+', &known),
            Some(Some("Home Reno".into()))
        );
        assert_eq!(named("@work", '@', &known), Some(Some("Work".into())));
        assert_eq!(
            named("+garden", '+', &known),
            Some(None),
            "unknown is still a mark"
        );
        assert_eq!(named("+1", '+', &known), None);
        assert_eq!(named("+", '+', &known), None);
        assert_eq!(named("@work", '+', &known), None);
    }

    #[test]
    fn priorities() {
        assert_eq!(priority("p1"), Some(1));
        assert_eq!(priority("P4"), Some(4));
        assert_eq!(priority("p0"), None);
        assert_eq!(priority("p5"), None);
    }

    #[test]
    fn durations() {
        for (w, min) in [
            ("30m", 30),
            ("45min", 45),
            ("2h", 120),
            ("1h30m", 90),
            ("1.5h", 90),
            ("1h30min", 90),
            ("2H", 120),
        ] {
            assert_eq!(duration(w), Some(min), "{w}");
        }
    }

    #[test]
    fn not_durations() {
        for w in [
            "0m", "m", "h", "30", "1.5m", "h30m", "-2h", "200h", "1h30", "2hours", "1984",
        ] {
            assert_eq!(duration(w), None, "{w}");
        }
    }
}
