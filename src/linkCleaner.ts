import { invoke } from "@tauri-apps/api/core";

export type Segment = {
  text: string;
  tracking: boolean;
  /** Not in the original: the link a redirect leads to, right after the redirect it replaces. */
  inserted?: boolean;
};

export type Analysis = {
  /** The original text split into pieces; `tracking` pieces are removed when cleaning. */
  segments: Segment[];
  /** The text with tracking parameters removed. */
  cleaned: string;
  trackingCount: number;
  /** Redirect links replaced by the link they lead to. */
  redirectCount: number;
};

/** True if cleaning changes anything. */
export function hasTracking(analysis: Analysis) {
  return analysis.trackingCount > 0 || analysis.redirectCount > 0;
}

/** Finds tracking parameters in links inside `text` (done in Rust: crates/link-cleaner). */
export function cleanText(text: string): Promise<Analysis> {
  return invoke<Analysis>("clean_text", { text });
}
