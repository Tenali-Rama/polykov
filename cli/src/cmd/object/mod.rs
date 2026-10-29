//! Object commands (`new`, `show`, `ls`, `edit`, `rm`, `pin`/`unpin`/`archive`/`unarchive`)
//! and the rendering and argument parsing they share.

use serde_json::{Number, Value};

use super::Output;

pub mod edit;
pub mod flag;
pub mod ls;
pub mod new;
pub mod rm;
pub mod show;

/// The API path of one object.
fn path(id: u64) -> String {
    format!("/api/objects/{id}")
}

/// Parses `--prop k=v`: `v` becomes an integer or float, then `true`/`false`, else a string.
pub fn parse_prop(arg: &str) -> Result<(String, Value), String> {
    let (key, raw) = arg
        .split_once('=')
        .ok_or_else(|| format!("expected key=value, got {arg:?}"))?;
    if key.is_empty() {
        return Err(format!("missing key in {arg:?}"));
    }
    let value = if let Ok(n) = raw.parse::<i64>() {
        Value::from(n)
    } else if let Some(n) = raw.parse::<f64>().ok().and_then(Number::from_f64) {
        Value::Number(n)
    } else {
        match raw {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => Value::String(raw.to_owned()),
        }
    };
    Ok((key.to_owned(), value))
}

/// `human` as the text; the object itself under `--json`.
fn output(human: String, object: Value) -> Output {
    Output {
        human,
        json: object,
    }
}

/// `#a #b` for an object's tags, empty if it has none.
fn tags(object: &Value) -> String {
    let tags = object["tags"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    tags.iter()
        .map(|t| format!("#{}", text(t)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A JSON value as text: strings without quotes, everything else as JSON.
fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// One `ls` line: `<id>  <type>  <title>  #tag ...`, the id prefixed with `*` if pinned.
fn line(object: &Value) -> String {
    let pin = if object["pinned"] == true { "*" } else { "" };
    let mut line = format!(
        "{pin}{}  {}  {}",
        object["id"],
        text(&object["type"]),
        text(&object["title"])
    );
    let tags = tags(object);
    if !tags.is_empty() {
        line.push_str("  ");
        line.push_str(&tags);
    }
    line
}

/// The `show` view: the title, then one aligned row per field and per property (`.key`).
fn detail(object: &Value) -> String {
    let yes_no = |v: &Value| if *v == true { "yes" } else { "no" }.to_owned();
    let mut rows = vec![
        ("id".to_owned(), object["id"].to_string()),
        ("type".to_owned(), text(&object["type"])),
        ("tags".to_owned(), tags(object)),
        ("pinned".to_owned(), yes_no(&object["pinned"])),
        ("archived".to_owned(), yes_no(&object["archived"])),
        ("created".to_owned(), text(&object["created"])),
        ("updated".to_owned(), text(&object["updated"])),
    ];
    if let Some(props) = object["properties"].as_object() {
        rows.extend(props.iter().map(|(k, v)| (format!(".{k}"), text(v))));
    }
    let width = rows
        .iter()
        .map(|(k, _)| k.chars().count())
        .max()
        .unwrap_or(0);
    let mut out = text(&object["title"]);
    for (key, value) in rows {
        let row = format!("\n  {key:width$}  {value}");
        out.push_str(row.trim_end());
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn prop_values_are_typed() {
        assert_eq!(parse_prop("n=3"), Ok(("n".into(), json!(3))));
        assert_eq!(parse_prop("x=-1.5"), Ok(("x".into(), json!(-1.5))));
        assert_eq!(parse_prop("ok=true"), Ok(("ok".into(), json!(true))));
        assert_eq!(parse_prop("ok=False"), Ok(("ok".into(), json!("False"))));
        assert_eq!(parse_prop("s=a=b"), Ok(("s".into(), json!("a=b"))));
        assert_eq!(parse_prop("e="), Ok(("e".into(), json!(""))));
        assert_eq!(parse_prop("f=inf"), Ok(("f".into(), json!("inf"))));
    }

    #[test]
    fn prop_needs_a_key_and_an_equals_sign() {
        assert!(parse_prop("novalue").is_err());
        assert!(parse_prop("=3").is_err());
    }

    #[test]
    fn line_marks_pinned_and_lists_tags() {
        let o =
            json!({"id": 7, "type": "note", "title": "Hi", "pinned": true, "tags": ["a", "b/c"]});
        assert_eq!(line(&o), "*7  note  Hi  #a #b/c");
        let o = json!({"id": 8, "type": "task", "title": "Yo", "pinned": false, "tags": []});
        assert_eq!(line(&o), "8  task  Yo");
    }
}
