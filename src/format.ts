/** "www.youtube.com/watch?…" → "youtube.com". Falls back to the input if it isn't a URL. */
export function hostOf(url: string) {
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** A saved-at time (ms since the epoch) as a short local date. */
export function formatDate(ms: number) {
  return new Date(ms).toLocaleDateString();
}

/**
 * A readable title from a link, for bookmarks without one (no preview yet, previews off, or a
 * site that gives none), from the end of its path with dashes and underscores as spaces:
 * "…/comments/1xh5k36/test_whether_an_option/" → "Test whether an option",
 * "…/binance-hack-report.html" → "Binance hack report", "github.com/rust-lang/rust/pull/120000"
 * → "Rust pull 120000". A bare domain gives the domain.
 */
export function titleFromLink(url: string) {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return url;
  }
  const parts = parsed.pathname
    .split("/")
    .map((part) => safeDecode(part).replace(/\.(html?|php|aspx?)$/i, ""))
    .filter(Boolean);
  if (parts.length === 0) return parsed.host.replace(/^www\./, "");

  const words = (part: string) => part.replace(/[-_+]+/g, " ").trim();
  const n = parts.length;
  const last = parts[n - 1];
  // Letters and digits mixed, no separators: an ID ("1xh5k36", "dQw4w9WgXcQ").
  const isId = (part: string) => /^[a-z0-9]+$/i.test(part) && /\d/.test(part) && /\p{L}/u.test(part);

  let title: string;
  if (/^\d+$/.test(last) && n >= 2) {
    // "…/rust/pull/120000": the number, what it is, and what it belongs to.
    title = [n >= 3 ? parts[n - 3] : null, parts[n - 2], last].filter(Boolean).map((p) => words(p!)).join(" ");
  } else if (isId(last)) {
    const readable = parts.slice(0, -1).reverse().find((part) => !isId(part) && /\p{L}{2,}/u.test(part));
    title = words(readable ?? last);
  } else {
    title = words(last);
  }
  return title.charAt(0).toUpperCase() + title.slice(1);
}

function safeDecode(part: string) {
  try {
    return decodeURIComponent(part);
  } catch {
    return part;
  }
}
