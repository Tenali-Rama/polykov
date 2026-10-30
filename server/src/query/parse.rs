//! Text to [`Query`]: split into terms, then read each term's field and values.

use chrono::NaiveDate;

use super::{
    Archived, Cmp, DateField, DateValue, Filter, Op, ParseError, Query, Ref, Status, TagPattern,
    Term, Unit,
};
use crate::objects::validate::TYPES;

/// One input char: its char offset, the char, and whether it sat inside `"quotes"`.
type Ch = (usize, char, bool);

/// A space-separated term, quotes removed; `start` is the offset of its first char.
struct Token {
    start: usize,
    chars: Vec<Ch>,
}

fn err(offset: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        message: message.into(),
        offset,
    }
}

/// Parses a filter query (grammar in ADR 0015). Terms are separated by whitespace and ANDed;
/// a leading `-` negates one; `"quotes"` keep spaces, colons and commas literal.
pub fn parse(input: &str) -> Result<Query, ParseError> {
    let mut query = Query {
        terms: Vec::new(),
        archived: Archived::No,
    };
    let mut archived_at = None;
    for token in lex(input)? {
        let (negated, body, at) = match token.chars.first() {
            Some(&(at, '-', false)) => (true, &token.chars[1..], at + 1),
            _ => (false, &token.chars[..], token.start),
        };
        if body.is_empty() {
            return Err(err(at, "empty term"));
        }
        let Some(colon) = body.iter().position(|&(_, c, q)| c == ':' && !q) else {
            query.terms.push(Term {
                negated,
                filter: Filter::Text(text(body)),
            });
            continue;
        };
        let name = text(&body[..colon]);
        let value_at = body[colon].0 + 1;
        let items = split_items(&body[colon + 1..], value_at, &name)?;
        if name == "archived" {
            if archived_at.replace(at).is_some() {
                return Err(err(at, "archived given twice"));
            }
            query.archived = archived(&items, negated)?;
            continue;
        }
        let filter = field(&name, &items, at)?;
        query.terms.push(Term { negated, filter });
    }
    Ok(query)
}

fn lex(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut current: Option<Token> = None;
    let mut quote = None;
    for (i, c) in input.chars().enumerate() {
        if c.is_whitespace() && quote.is_none() {
            tokens.extend(current.take());
            continue;
        }
        let token = current.get_or_insert(Token {
            start: i,
            chars: Vec::new(),
        });
        match (c, quote) {
            ('"', None) => quote = Some(i),
            ('"', Some(_)) => quote = None,
            _ => token.chars.push((i, c, quote.is_some())),
        }
    }
    if let Some(at) = quote {
        return Err(err(at, "unclosed quote"));
    }
    tokens.extend(current);
    Ok(tokens)
}

fn text(chars: &[Ch]) -> String {
    chars.iter().map(|&(_, c, _)| c).collect()
}

/// One comma-list item of a field value, operator not yet read.
struct Item<'a> {
    at: usize,
    chars: &'a [Ch],
}

/// Splits a field value at its unquoted commas; never returns an empty list.
fn split_items<'a>(value: &'a [Ch], at: usize, name: &str) -> Result<Vec<Item<'a>>, ParseError> {
    let mut items = Vec::new();
    let mut rest = value;
    let mut item_at = at;
    loop {
        let end = rest
            .iter()
            .position(|&(_, c, q)| c == ',' && !q)
            .unwrap_or(rest.len());
        let part = &rest[..end];
        let Some(&(first, _, _)) = part.first() else {
            return Err(err(item_at, format!("empty value for {name}")));
        };
        items.push(Item {
            at: first,
            chars: part,
        });
        let Some(&(comma, _, _)) = rest.get(end) else {
            return Ok(items);
        };
        item_at = comma + 1;
        rest = &rest[end + 1..];
    }
}

impl Item<'_> {
    /// The leading unquoted operator, if any, and the value after it.
    fn split_op(&self, name: &str) -> Result<(Option<Op>, usize, String, bool), ParseError> {
        let unquoted =
            |i: usize, c: char| matches!(self.chars.get(i), Some(&(_, x, false)) if x == c);
        let (op, len) = match () {
            _ if unquoted(0, '<') && unquoted(1, '=') => (Some(Op::Le), 2),
            _ if unquoted(0, '>') && unquoted(1, '=') => (Some(Op::Ge), 2),
            _ if unquoted(0, '<') => (Some(Op::Lt), 1),
            _ if unquoted(0, '>') => (Some(Op::Gt), 1),
            _ if unquoted(0, '=') => (Some(Op::Eq), 1),
            _ => (None, 0),
        };
        let rest = &self.chars[len..];
        let Some(&(at, _, _)) = rest.first() else {
            return Err(err(self.at + len, format!("empty value for {name}")));
        };
        let quoted = rest.iter().any(|&(_, _, q)| q);
        Ok((op, at, text(rest), quoted))
    }

    /// The value of a field that takes no operator.
    fn plain(&self, name: &str) -> Result<(usize, String, bool), ParseError> {
        match self.split_op(name)? {
            (Some(_), ..) => Err(err(self.at, format!("{name} takes no operator"))),
            (None, at, value, quoted) => Ok((at, value, quoted)),
        }
    }
}

fn field(name: &str, items: &[Item], at: usize) -> Result<Filter, ParseError> {
    Ok(match name {
        "type" => Filter::Type(each(items, |item| {
            let (at, value, _) = item.plain(name)?;
            let value = value.to_lowercase();
            if TYPES.contains(&value.as_str()) {
                Ok(value)
            } else {
                Err(err(at, format!("unknown type {value}")))
            }
        })?),
        "tag" => Filter::Tag(each(items, |item| tag(item.plain(name)?))?),
        "status" => Filter::Status(each(items, |item| {
            let (at, value, _) = item.plain(name)?;
            match value.to_lowercase().as_str() {
                "open" => Ok(Status::Open),
                "done" => Ok(Status::Done),
                _ => Err(err(at, "status must be open or done")),
            }
        })?),
        "priority" => Filter::Priority(each(items, |item| {
            let (op, at, value, _) = item.split_op(name)?;
            match value.parse::<u8>() {
                Ok(p @ 1..=4) => Ok(Cmp {
                    op: op.unwrap_or(Op::Eq),
                    value: p,
                }),
                _ => Err(err(at, "priority must be 1 to 4")),
            }
        })?),
        "due" | "scheduled" | "created" | "updated" | "completed" => {
            Filter::Date(date_field(name), each(items, |item| date(item, name))?)
        }
        "project" => Filter::Project(each(items, |item| reference(item.plain(name)?, name))?),
        "area" => Filter::Area(each(items, |item| reference(item.plain(name)?, name))?),
        "pinned" => Filter::Pinned(boolean(single(items, name)?.plain(name)?, name)?),
        _ => return Err(err(at, format!("unknown field {name:?}"))),
    })
}

/// Reads every item of a comma list.
fn each<T>(
    items: &[Item],
    read: impl Fn(&Item) -> Result<T, ParseError>,
) -> Result<Vec<T>, ParseError> {
    items.iter().map(read).collect()
}

fn single<'a, 'b>(items: &'a [Item<'b>], name: &str) -> Result<&'a Item<'b>, ParseError> {
    match items {
        [item] => Ok(item),
        [_, second, ..] => Err(err(second.at, format!("{name} takes one value"))),
        // `split_items` never returns an empty list.
        [] => Err(err(0, format!("empty value for {name}"))),
    }
}

fn boolean((at, value, _): (usize, String, bool), name: &str) -> Result<bool, ParseError> {
    match value.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(err(at, format!("{name} must be true or false"))),
    }
}

fn archived(items: &[Item], negated: bool) -> Result<Archived, ParseError> {
    let (at, value, quoted) = single(items, "archived")?.plain("archived")?;
    if value.eq_ignore_ascii_case("any") {
        return match negated {
            false => Ok(Archived::Any),
            true => Err(err(at, "archived:any cannot be negated")),
        };
    }
    let yes = boolean((at, value, quoted), "archived").map_err(|e| ParseError {
        message: "archived must be true, false or any".into(),
        ..e
    })?;
    Ok(if yes != negated {
        Archived::Yes
    } else {
        Archived::No
    })
}

fn tag((at, value, _): (usize, String, bool)) -> Result<TagPattern, ParseError> {
    let (tag, subtree) = match value.strip_suffix("/*") {
        Some(parent) => (parent, true),
        None => (value.as_str(), false),
    };
    let tag = crate::tags::normalize(tag).map_err(|_| err(at, format!("invalid tag {value}")))?;
    Ok(TagPattern { tag, subtree })
}

fn reference((at, value, quoted): (usize, String, bool), name: &str) -> Result<Ref, ParseError> {
    if quoted {
        let title = value.trim();
        if title.is_empty() {
            return Err(err(at, format!("empty value for {name}")));
        }
        return Ok(Ref::Title(title.to_string()));
    }
    match value.parse::<i64>() {
        Ok(id) if id > 0 => Ok(Ref::Id(id)),
        _ => Err(err(at, format!("{name} takes an id or a \"quoted title\""))),
    }
}

fn date_field(name: &str) -> DateField {
    match name {
        "scheduled" => DateField::Scheduled,
        "created" => DateField::Created,
        "updated" => DateField::Updated,
        "completed" => DateField::Completed,
        _ => DateField::Due,
    }
}

fn date(item: &Item, name: &str) -> Result<Cmp<DateValue>, ParseError> {
    let (op, at, value, _) = item.split_op(name)?;
    let value = value.to_lowercase();
    let bad = || {
        err(
            at,
            format!(
                "invalid date {value}: use today, tomorrow, yesterday, YYYY-MM-DD, Nd, Nw, Nm or none"
            ),
        )
    };
    let parsed = match value.as_str() {
        "today" => DateValue::Relative(0, Unit::Day),
        "tomorrow" => DateValue::Relative(1, Unit::Day),
        "yesterday" => DateValue::Relative(-1, Unit::Day),
        "none" if op.is_some_and(|op| op != Op::Eq) => {
            return Err(err(item.at, format!("{name}:none takes no operator")));
        }
        "none" => DateValue::None,
        _ if value.contains('-') && value.len() == 10 => {
            DateValue::Absolute(NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| bad())?)
        }
        _ => {
            let unit = match value.chars().last() {
                Some('d') => Unit::Day,
                Some('w') => Unit::Week,
                Some('m') => Unit::Month,
                _ => return Err(bad()),
            };
            let number = &value[..value.len() - 1];
            let digits = number.strip_prefix(['-', '+']).unwrap_or(number);
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            DateValue::Relative(number.parse().map_err(|_| bad())?, unit)
        }
    };
    Ok(Cmp {
        op: op.unwrap_or(Op::Eq),
        value: parsed,
    })
}
