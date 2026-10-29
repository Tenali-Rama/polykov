//! Hierarchical tags (spec §3): `work/clienta/design` is a tag inside the folders `work` and
//! `work/clienta`. Input rules, the `object_tags` store and `GET /api/tags`.

pub mod store;

use axum::{Json, Router, extract::State, routing::get};
use serde_json::{Value, json};

use crate::{
    auth::Session,
    db::Db,
    http::{ApiError, blocking},
};

/// Maximum tag length after normalizing, in characters.
pub const MAX_TAG_CHARS: usize = 100;
/// Maximum number of distinct tags on one object.
pub const MAX_TAGS: usize = 50;

/// The tag routes, merged by `app::router`.
pub fn routes() -> Router<Db> {
    Router::new().route("/api/tags", get(list))
}

/// Normalizes `input`: trims it, strips one leading `#` and lowercases it. The result must be
/// 1 to [`MAX_TAG_CHARS`] characters of `/`-separated segments, each non-empty and free of
/// whitespace, `#` and `,`. Fails with `invalid tag: <input>`.
pub fn normalize(input: &str) -> Result<String, ApiError> {
    let trimmed = input.trim();
    let tag = trimmed.strip_prefix('#').unwrap_or(trimmed).to_lowercase();
    let segment_ok =
        |s: &str| !s.is_empty() && !s.chars().any(|c| c.is_whitespace() || c == '#' || c == ',');
    if tag.chars().count() > MAX_TAG_CHARS || !tag.split('/').all(segment_ok) {
        return Err(ApiError::bad_request(format!("invalid tag: {input}")));
    }
    Ok(tag)
}

/// Normalizes every tag in `inputs` and returns the distinct results, sorted. Fails on the
/// first invalid tag, or if more than [`MAX_TAGS`] remain.
pub fn normalize_all(inputs: &[String]) -> Result<Vec<String>, ApiError> {
    let mut tags = inputs
        .iter()
        .map(|t| normalize(t))
        .collect::<Result<Vec<_>, _>>()?;
    tags.sort();
    tags.dedup();
    if tags.len() > MAX_TAGS {
        return Err(ApiError::bad_request(format!(
            "an object can have at most {MAX_TAGS} tags"
        )));
    }
    Ok(tags)
}

async fn list(State(db): State<Db>, _: Session) -> Result<Json<Value>, ApiError> {
    let tags = blocking(move || store::counts(&db)).await?;
    Ok(Json(json!({ "tags": tags })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_hash_trims_and_lowercases() {
        assert_eq!(normalize("  #Work/ClientA ").unwrap(), "work/clienta");
        assert_eq!(normalize("ÄRGER").unwrap(), "ärger");
    }

    #[test]
    fn normalize_rejects_bad_segments_and_length() {
        for bad in ["", "#", "a//b", "/a", "a/", "a b", "a/#b", "a,b", "##a"] {
            assert!(normalize(bad).is_err(), "{bad:?}");
        }
        assert!(normalize(&"a".repeat(MAX_TAG_CHARS)).is_ok());
        assert!(normalize(&"a".repeat(MAX_TAG_CHARS + 1)).is_err());
    }

    #[test]
    fn normalize_all_dedups_and_caps() {
        let input = ["b", "#A", "a "].map(String::from);
        assert_eq!(normalize_all(&input).unwrap(), ["a", "b"]);
        let many: Vec<String> = (0..=MAX_TAGS).map(|i| format!("t{i}")).collect();
        assert!(normalize_all(&many).is_err());
        assert!(normalize_all(&many[..MAX_TAGS]).is_ok());
    }
}
