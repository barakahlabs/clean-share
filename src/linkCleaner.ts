import { invoke } from "@tauri-apps/api/core";

export type Segment = { text: string; tracking: boolean };

export type Analysis = {
  /** The original text split into pieces; `tracking` pieces are removed when cleaning. */
  segments: Segment[];
  /** The text with tracking parameters removed. */
  cleaned: string;
  trackingCount: number;
};

/** Finds tracking parameters in links inside `text` (done in Rust: crates/link-cleaner). */
export function cleanText(text: string): Promise<Analysis> {
  return invoke<Analysis>("clean_text", { text });
}
