-- Schema v2: more of what link previews find (crates/link-preview). Title, description, site
-- name, image and embed already had columns in v1.
--
-- - page_kind: what the page is, from JSON-LD or og:type ("recipe", "article", "video", ...).
-- - keywords: the page's declared keywords, one per line ('' = none).
-- - excerpt: the start of the page's main text, for tag suggestions and on-device AI.

ALTER TABLE bookmark ADD COLUMN page_kind TEXT;
ALTER TABLE bookmark ADD COLUMN keywords TEXT NOT NULL DEFAULT '';
ALTER TABLE bookmark ADD COLUMN excerpt TEXT;
ALTER TABLE bookmark ADD COLUMN author TEXT;
ALTER TABLE bookmark ADD COLUMN published TEXT;
ALTER TABLE bookmark ADD COLUMN icon_url TEXT;

-- The background fetcher takes pending bookmarks, newest first.
CREATE INDEX bookmark_preview_queue ON bookmark (preview_status, created_at DESC);
