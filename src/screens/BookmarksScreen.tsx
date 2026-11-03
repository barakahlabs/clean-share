import { useEffect, useRef, useState } from "react";
import {
  getBookmark,
  listBookmarks,
  listTags,
  onPreview,
  type Bookmark,
  type TagCount,
} from "../bookmarks";
import { BookmarkCard } from "../components/BookmarkCard";
import { BookmarkDetailScreen } from "./BookmarkDetailScreen";
import "./Screen.css";
import "./BookmarksScreen.css";

/** How long typing has to pause before searching. */
const SEARCH_DELAY_MS = 200;

/**
 * Saved bookmarks as a grid of cards (two per row on a phone): newest first, or by relevance
 * while searching. A search box (as you type)
 * and tag chips (all selected tags must match) narrow the list. Tapping a bookmark opens its
 * detail screen; Android's back button (or "‹ Bookmarks") returns to the list. With link
 * previews on, each card shows its thumbnail, updated as previews arrive.
 */
export function BookmarksScreen() {
  const [items, setItems] = useState<Bookmark[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [search, setSearch] = useState("");
  const [tags, setTags] = useState<TagCount[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  // Only the newest request may update the list, so a slow older search can't overwrite it.
  const latest = useRef(0);

  const load = (cursor: string | null) => {
    const request = ++latest.current;
    return listBookmarks({ search: search || null, tags: selected, cursor })
      .then((page) => {
        if (request !== latest.current) return;
        setItems((old) => (cursor && old ? [...old, ...page.items] : page.items));
        setNext(page.next);
        setError(null);
      })
      .catch((e) => request === latest.current && setError(String(e)));
  };

  // Search once typing pauses.
  useEffect(() => {
    const timer = setTimeout(() => setSearch(query.trim()), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [query]);

  useEffect(() => {
    load(null);
  }, [search, selected]);

  const reloadTags = () =>
    listTags()
      .then((all) => setTags(all.filter((tag) => tag.count > 0)))
      .catch(() => setTags([]));

  useEffect(() => {
    reloadTags();
  }, []);

  // A preview finished in the background: refresh that card.
  useEffect(
    () =>
      onPreview((id) => {
        getBookmark(id)
          .then((fresh) => {
            if (fresh) setItems((old) => old?.map((b) => (b.id === id ? fresh : b)) ?? old);
          })
          .catch(() => {});
      }),
    [],
  );

  const toggleTag = (name: string) =>
    setSelected((old) => (old.includes(name) ? old.filter((t) => t !== name) : [...old, name]));

  const filtering = search !== "" || selected.length > 0;
  const clearFilters = () => {
    setQuery("");
    setSearch("");
    setSelected([]);
  };

  // Back from the detail screen returns here. Sheets opened inside it (edit, delete confirm)
  // have their own history entries; closing one lands back on the detail entry, so stay there.
  useEffect(() => {
    const onBack = (event: PopStateEvent) => {
      const page = event.state?.bookmarksPage;
      if (page !== "detail" && page !== "edit") setOpenId(null);
    };
    window.addEventListener("popstate", onBack);
    return () => window.removeEventListener("popstate", onBack);
  }, []);

  const open = (id: string) => {
    history.pushState({ bookmarksPage: "detail" }, "");
    setOpenId(id);
  };

  if (openId) {
    return (
      <BookmarkDetailScreen
        id={openId}
        onBack={() => history.back()}
        onDeleted={() => {
          history.back();
          load(null);
          reloadTags();
        }}
      />
    );
  }

  return (
    <main className="screen with-tabs">
      <h1>Bookmarks</h1>

      {(Boolean(items?.length) || filtering) && (
        <div className="bookmark-filters">
          <input
            type="search"
            className="bookmark-search"
            value={query}
            placeholder="Search bookmarks"
            enterKeyHint="search"
            autoCapitalize="none"
            aria-label="Search bookmarks"
            onChange={(event) => setQuery(event.target.value)}
          />
          {tags.length > 0 && (
            <div className="tag-filter" role="group" aria-label="Filter by tag">
              {tags.map((tag) => (
                <button
                  key={tag.name}
                  type="button"
                  className={selected.includes(tag.name) ? "tag-filter-chip on" : "tag-filter-chip"}
                  aria-pressed={selected.includes(tag.name)}
                  onClick={() => toggleTag(tag.name)}
                >
                  {tag.name}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {error ? (
        <p className="screen-empty">{error}</p>
      ) : items === null ? null : items.length === 0 && filtering ? (
        <p className="screen-empty">
          No bookmarks match.{" "}
          <button type="button" className="text-button" onClick={clearFilters}>
            Clear filters
          </button>
        </p>
      ) : items.length === 0 ? (
        <p className="screen-empty">
          No bookmarks yet. Share a link from any app and pick Save bookmark.
        </p>
      ) : (
        <>
          <ul className="bookmark-grid">
            {items.map((bookmark) => (
              <li key={bookmark.id}>
                <BookmarkCard bookmark={bookmark} onOpen={() => open(bookmark.id)} />
              </li>
            ))}
          </ul>
          {next && (
            <button type="button" className="text-button load-more" onClick={() => load(next)}>
              Load more
            </button>
          )}
        </>
      )}
    </main>
  );
}
