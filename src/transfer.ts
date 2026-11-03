import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// Export and import (src-tauri/src/transfer.rs → crates/bookmarks transfer).

export type ExportFormat = "json" | "html";

export type ImportReport = {
  format: "cleanshare" | "html" | "pocket" | "raindrop" | "csv";
  /** New bookmarks. */
  added: number;
  /** Already saved: tags added, empty title and note filled. */
  merged: number;
  /** Not a link that can be saved. */
  skipped: number;
};

/** Writes every bookmark to a file and opens the share sheet with it. */
export function exportBookmarks(format: ExportFormat): Promise<void> {
  return invoke("bookmarks_export", { format });
}

/** Lets the user pick an export file and imports it; `null` if they cancelled. */
export async function importBookmarks(): Promise<ImportReport | null> {
  const path = await open({ multiple: false, directory: false });
  if (!path) return null;
  return invoke("bookmarks_import", { path });
}
