# 0008 — Attachment storage and download

- **Status**: accepted
- **Context**: Task 016 adds file attachments to any object (spec §3). Files are user data that the browser may render, so storage and download must be safe by default.
- **Decision**:
  - Upload is the raw request body (`POST /api/objects/{id}/attachments?name=...`, `Content-Type` = mime): no multipart parser, and the CLI can stream a file with one call.
  - Files are stored as `<data dir>/attachments/<id>`, never under the user's name, so names need no path sanitising beyond rejecting `/`, `\` and control chars. Each write goes to `<id>.tmp`, is synced and renamed, inside the row's transaction: a failed write leaves no row.
  - Uploads are capped at 25 MiB (`attachments::MAX_BYTES`, owner may change it); over gets `413`.
  - Downloads always send `Content-Disposition: attachment` with an RFC 8187 `filename*` and `X-Content-Type-Options: nosniff`. `?inline=1` is honoured only for `image/*` except SVG, since SVG can run scripts on our origin.
- **Consequences**: Purging an object must call `attachments::remove_files` before the `DELETE` (rows cascade, files don't). Don't serve attachments inline by mime type alone, and don't name files after user input.
