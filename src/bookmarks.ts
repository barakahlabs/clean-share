import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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
  /** From the preview: "recipe", "article", "video"… */
  pageKind: string | null;
  keywords: string[];
  author: string | null;
  published: string | null;
  iconUrl: string | null;
  excerpt: string | null;
  previewStatus: "none" | "pending" | "done" | "failed";
  previewFetchedAt: number | null;
  rulesRevision: string;
  tags: BookmarkTag[];
  /** Milliseconds since the Unix epoch. */
  createdAt: number;
  updatedAt: number;
};

export type Link = { original: string; cleaned: string; provider: string | null };

/** What the save screen shows before saving. `title` is suggested from the shared text. */
export type Prepared = { links: Link[]; existing: Bookmark | null; title: string | null };

export type NewBookmark = {
  sharedText: string;
  title?: string | null;
  note?: string | null;
  tags?: TagInput[];
};

/** `created` is false when the link was already saved and the new tags were merged in. */
export type Saved = { bookmark: Bookmark; created: boolean };

/** Fields to change. A missing field is left as is; `""` clears it. */
export type BookmarkPatch = { title?: string; note?: string };

export type ListQuery = {
  search?: string | null;
  tags?: string[];
  /** `next` from the previous page. */
  cursor?: string | null;
  limit?: number;
};

export type Page<T> = { items: T[]; next: string | null };

/** The bookmark on the save sheet, as it is now. `picked` tags are never suggested. */
export type TagDraft = { text: string; title?: string | null; picked: string[] };

/** Why a tag was suggested: named in the content, used on the same site, used together with a
 * picked tag, or from the built-in provider map (YouTube → video). */
export type SuggestionReason = "content" | "site" | "related" | "provider";
export type Suggestion = { name: string; reason: SuggestionReason; score: number };

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

export function updateBookmark(id: string, patch: BookmarkPatch): Promise<Bookmark> {
  return invoke("bookmark_update", { id, patch });
}

/** Replaces a bookmark's tags; new names are created. */
export function setBookmarkTags(id: string, tags: TagInput[]): Promise<BookmarkTag[]> {
  return invoke("bookmark_set_tags", { id, tags });
}

export function deleteBookmark(id: string): Promise<void> {
  return invoke("bookmark_delete", { id });
}

/** Every tag with its bookmark count, most used first. */
export function listTags(): Promise<TagCount[]> {
  return invoke("tag_list");
}

/** What a link preview found (crates/link-preview). Links are already cleaned. */
export type LinkPreview = {
  url: string;
  title: string | null;
  description: string | null;
  siteName: string | null;
  imageUrl: string | null;
  iconUrl: string | null;
  author: string | null;
  kind: string | null;
  keywords: string[];
  published: string | null;
  language: string | null;
  excerpt: string | null;
  embed: { kind: string; url: string | null } | null;
};

/** The preview for the first link in shared text, before saving; `null` with previews off. */
export function fetchPreview(text: string): Promise<LinkPreview | null> {
  return invoke("preview_fetch", { text });
}

/** Queues a bookmark's preview again (e.g. after it failed). */
export function retryPreview(id: string): Promise<void> {
  return invoke("bookmark_preview_retry", { id });
}

/** The bookmark's thumbnail on the phone, or null if it has none. The fetch time in the URL
 * makes a retried preview's new thumbnail show instead of a cached old one. */
export function thumbnailUrl(bookmark: Bookmark): string | null {
  if (bookmark.previewStatus !== "done" || !bookmark.imageUrl) return null;
  return `${convertFileSrc(`${bookmark.id}.jpg`, "thumb")}?v=${bookmark.previewFetchedAt ?? 0}`;
}

/** The site's icon saved on the phone, for bookmarks without a picture. The file may not
 * exist (a site without one, or the bookmark has a picture instead): hide the image on error. */
export function siteIconUrl(bookmark: Bookmark): string | null {
  if (bookmark.previewStatus !== "done") return null;
  return `${convertFileSrc(`${bookmark.id}-icon.png`, "thumb")}?v=${bookmark.previewFetchedAt ?? 0}`;
}

/** Calls `onDone` with a bookmark's id whenever its preview is fetched (or fails). */
export function onPreview(onDone: (id: string) => void): () => void {
  const stop = listen<string>("bookmark-preview", (event) => onDone(event.payload));
  return () => {
    stop.then((unlisten) => unlisten()).catch(() => {});
  };
}

/** Tags to suggest for the bookmark on the save sheet, best first. */
export function suggestTags(draft: TagDraft): Promise<Suggestion[]> {
  return invoke("tag_suggest", { draft });
}

/** Renames a tag; if a tag named `to` exists, the two are merged. */
export function renameTag(from: string, to: string): Promise<void> {
  return invoke("tag_rename", { from, to });
}

/** Deletes a tag, removing it from every bookmark that has it. */
export function deleteTag(name: string): Promise<void> {
  return invoke("tag_delete", { name });
}
