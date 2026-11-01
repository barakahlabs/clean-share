import { invoke } from "@tauri-apps/api/core";

// Typed wrappers for the bookmarks commands (src-tauri/src/bookmarks.rs → crates/bookmarks).
// Shapes match the Rust types in crates/bookmarks/src/model.rs (serde camelCase).

export type TagSource = "user" | "suggested" | "ai";
export type BookmarkTag = { name: string; source: TagSource };
export type TagInput = { name: string; source?: TagSource };
/** A tag and how many bookmarks use it (0 for tags no bookmark uses any more). */
export type TagCount = { name: string; count: number };

export type Bookmark = {
  id: string;
  /** Cleaned link. */
  url: string;
  /** As shared, with tracking. Not shown by default. */
  originalUrl: string;
  title: string | null;
  note: string | null;
  sharedText: string;
  provider: string | null;
  siteName: string | null;
  description: string | null;
  imageUrl: string | null;
  embedKind: string | null;
  embedUrl: string | null;
  previewStatus: "none" | "pending" | "done" | "failed";
  previewFetchedAt: number | null;
  rulesRevision: string;
  tags: BookmarkTag[];
  /** Milliseconds since the Unix epoch. */
  createdAt: number;
  updatedAt: number;
};

export type Link = { original: string; cleaned: string; provider: string | null };

/** What the save screen shows before saving. */
export type Prepared = { links: Link[]; existing: Bookmark | null };

export type NewBookmark = {
  sharedText: string;
  title?: string | null;
  note?: string | null;
  tags?: TagInput[];
};

/** `created` is false when the link was already saved and the new tags were merged in. */
export type Saved = { bookmark: Bookmark; created: boolean };

export type ListQuery = {
  search?: string | null;
  tags?: string[];
  /** `next` from the previous page. */
  cursor?: string | null;
  limit?: number;
};

export type Page<T> = { items: T[]; next: string | null };

export function prepareBookmark(text: string): Promise<Prepared> {
  return invoke("bookmark_prepare", { text });
}

export function saveBookmark(bookmark: NewBookmark): Promise<Saved> {
  return invoke("bookmark_save", { bookmark });
}

export function listBookmarks(query: ListQuery = {}): Promise<Page<Bookmark>> {
  return invoke("bookmark_list", { query });
}

export function getBookmark(id: string): Promise<Bookmark | null> {
  return invoke("bookmark_get", { id });
}

export function deleteBookmark(id: string): Promise<void> {
  return invoke("bookmark_delete", { id });
}

/** Every tag with its bookmark count, most used first. */
export function listTags(): Promise<TagCount[]> {
  return invoke("tag_list");
}

/** Renames a tag; if a tag named `to` exists, the two are merged. */
export function renameTag(from: string, to: string): Promise<void> {
  return invoke("tag_rename", { from, to });
}

/** Deletes a tag, removing it from every bookmark that has it. */
export function deleteTag(name: string): Promise<void> {
  return invoke("tag_delete", { name });
}
