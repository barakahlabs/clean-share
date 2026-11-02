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
