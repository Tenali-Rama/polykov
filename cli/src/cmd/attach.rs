//! `polykov attach add|ls|get|rm`: files attached to objects.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use serde_json::json;

use crate::cmd::object::ls::encode;
use crate::cmd::{Ctx, Output};
use crate::error::Error;

#[derive(clap::Subcommand)]
pub enum Command {
    /// Upload a file and attach it to an object
    Add {
        id: u64,
        file: PathBuf,
        /// Name to store it under [default: the file's name]
        #[arg(long, value_name = "N")]
        name: Option<String>,
    },
    /// List an object's attachments, oldest first
    Ls { id: u64 },
    /// Download an attachment
    Get {
        aid: u64,
        /// Where to write it [default: its name, in the current dir]
        #[arg(short, value_name = "PATH")]
        o: Option<PathBuf>,
        /// Overwrite an existing file
        #[arg(long)]
        force: bool,
    },
    /// Delete an attachment and its file
    Rm { aid: u64 },
}

pub fn run(ctx: &Ctx, command: Command) -> Result<Output, Error> {
    match command {
        Command::Add { id, file, name } => add(ctx, id, &file, name),
        Command::Ls { id } => ls(ctx, id),
        Command::Get { aid, o, force } => get(ctx, aid, o, force),
        Command::Rm { aid } => rm(ctx, aid),
    }
}

fn add(ctx: &Ctx, id: u64, file: &Path, name: Option<String>) -> Result<Output, Error> {
    let shown = file.display();
    let bytes = std::fs::read(file).map_err(|e| Error::Msg(format!("cannot read {shown}: {e}")))?;
    let name = match name {
        Some(name) => name,
        None => file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Msg(format!("cannot name {shown}; use --name")))?
            .to_owned(),
    };
    let path = format!("/api/objects/{id}/attachments?name={}", encode(&name));
    let attachment = ctx.client().post_bytes(&path, mime(file), &bytes)?.ok()?;
    Ok(Output {
        human: format!("attached {}", attachment["id"]),
        json: attachment,
    })
}

fn ls(ctx: &Ctx, id: u64) -> Result<Output, Error> {
    let body = ctx
        .client()
        .get(&format!("/api/objects/{id}/attachments"))?
        .ok()?;
    let lines: Vec<String> = body["attachments"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|a| {
            format!(
                "{}  {}  {}  {}",
                a["id"],
                a["name"].as_str().unwrap_or_default(),
                human_size(a["size"].as_u64().unwrap_or(0)),
                a["mime"].as_str().unwrap_or_default()
            )
        })
        .collect();
    let human = if lines.is_empty() {
        "no attachments".to_owned()
    } else {
        lines.join("\n")
    };
    Ok(Output { human, json: body })
}

fn get(ctx: &Ctx, aid: u64, out: Option<PathBuf>, force: bool) -> Result<Output, Error> {
    let reply = ctx
        .client()
        .get_bytes(&format!("/api/attachments/{aid}"))?
        .ok()?;
    let target = match out {
        Some(path) => path,
        None => reply
            .headers
            .get("content-disposition")
            .and_then(|v| v.to_str().ok())
            .and_then(disposition_name)
            .map(PathBuf::from)
            .ok_or_else(|| Error::Msg("server sent no usable file name; use -o PATH".into()))?,
    };
    let shown = target.display();
    let mut options = OpenOptions::new();
    options.write(true);
    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }
    let mut file = options.open(&target).map_err(|e| match e.kind() {
        ErrorKind::AlreadyExists => Error::Msg(format!("{shown} exists; use --force to overwrite")),
        _ => Error::Msg(format!("cannot write {shown}: {e}")),
    })?;
    file.write_all(&reply.bytes)
        .map_err(|e| Error::Msg(format!("cannot write {shown}: {e}")))?;
    let size = reply.bytes.len();
    Ok(Output {
        human: format!("wrote {shown} ({})", human_size(size as u64)),
        json: json!({ "id": aid, "path": target, "size": size }),
    })
}

fn rm(ctx: &Ctx, aid: u64) -> Result<Output, Error> {
    ctx.client()
        .delete(&format!("/api/attachments/{aid}"))?
        .ok()?;
    Ok(Output {
        human: format!("removed {aid}"),
        json: json!({ "id": aid, "deleted": true }),
    })
}

/// The `Content-Type` for a file, from its extension; `application/octet-stream` if unknown.
fn mime(file: &Path) -> &'static str {
    let ext = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}

/// The file name in `attachment; filename*=UTF-8''<percent-encoded>`, if it is a plain name
/// (no path separators, not `.` or `..`).
fn disposition_name(header: &str) -> Option<String> {
    let (_, rest) = header.split_once("filename*=UTF-8''")?;
    let encoded = rest.split(';').next()?.trim();
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut iter = encoded.bytes();
    while let Some(b) = iter.next() {
        if b == b'%' {
            let hex = [iter.next()?, iter.next()?];
            bytes.push(u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?);
        } else {
            bytes.push(b);
        }
    }
    let name = String::from_utf8(bytes).ok()?;
    let plain = !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\']);
    plain.then_some(name)
}

/// `512 B`, `5.0 KiB`, `1.2 MiB`, ...
fn human_size(bytes: u64) -> String {
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    for unit in ["KiB", "MiB"] {
        if value < 1024.0 {
            return format!("{value:.1} {unit}");
        }
        value /= 1024.0;
    }
    format!("{value:.1} GiB")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_uses_the_extension_case_insensitively() {
        assert_eq!(mime(Path::new("a/Scan.PDF")), "application/pdf");
        assert_eq!(mime(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(mime(Path::new("data.bin")), "application/octet-stream");
        assert_eq!(mime(Path::new("Makefile")), "application/octet-stream");
    }

    #[test]
    fn disposition_name_decodes_and_rejects_paths() {
        assert_eq!(
            disposition_name("attachment; filename*=UTF-8''r%C3%A9sum%C3%A9%20v2.pdf").as_deref(),
            Some("résumé v2.pdf")
        );
        assert_eq!(
            disposition_name("attachment; filename*=UTF-8''..%2Fx"),
            None
        );
        assert_eq!(disposition_name("attachment; filename*=UTF-8''.."), None);
        assert_eq!(disposition_name("attachment; filename*=UTF-8''%zz"), None);
        assert_eq!(disposition_name("attachment"), None);
    }

    #[test]
    fn human_size_picks_a_binary_unit() {
        assert_eq!(human_size(0), "0 B");
        assert_eq!(human_size(1023), "1023 B");
        assert_eq!(human_size(5120), "5.0 KiB");
        assert_eq!(human_size(1_258_291), "1.2 MiB");
        assert_eq!(human_size(3 * 1024 * 1024 * 1024), "3.0 GiB");
    }
}
