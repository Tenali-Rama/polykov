//! Response headers for an attachment download: the stored type, a forced download (inline
//! only for raster images) and no MIME sniffing.

use axum::http::{HeaderMap, HeaderValue, header};

use super::store::Attachment;
use crate::http::ApiError;

/// Headers for sending `a`. `inline` asks for `Content-Disposition: inline`, which is honoured
/// only for `image/*` other than SVG (SVG can run scripts when opened directly).
pub fn download(a: &Attachment, inline: bool) -> Result<HeaderMap, ApiError> {
    let disposition = if inline && is_inline_image(&a.mime) {
        "inline"
    } else {
        "attachment"
    };
    let value = |s: &str| {
        HeaderValue::from_str(s)
            .map_err(|e| ApiError::Internal(format!("attachment {} header: {e}", a.id)))
    };
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, value(&a.mime)?);
    headers.insert(
        header::CONTENT_DISPOSITION,
        value(&format!(
            "{disposition}; filename*=UTF-8''{}",
            percent_encode(&a.name)
        ))?,
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    Ok(headers)
}

/// Whether `mime` is an image type safe to show inline.
fn is_inline_image(mime: &str) -> bool {
    let essence = mime
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    essence.starts_with("image/") && essence != "image/svg+xml"
}

/// Percent-encodes `s` as UTF-8, keeping only RFC 8187 `attr-char`s.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&b) {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encode_keeps_attr_chars_only() {
        assert_eq!(percent_encode("a-b_c.txt"), "a-b_c.txt");
        assert_eq!(
            percent_encode("my file;\"é\".pdf"),
            "my%20file%3B%22%C3%A9%22.pdf"
        );
    }

    #[test]
    fn inline_only_for_raster_images() {
        assert!(is_inline_image("image/png"));
        assert!(is_inline_image("IMAGE/JPEG; q=1"));
        assert!(!is_inline_image("image/svg+xml"));
        assert!(!is_inline_image("text/html"));
    }
}
