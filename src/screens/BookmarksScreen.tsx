import { useEffect, useState } from "react";
import { listBookmarks, type Bookmark } from "../bookmarks";
import { formatDate, hostOf } from "../format";
import { BookmarkDetailScreen } from "./BookmarkDetailScreen";
import "./Screen.css";
import "./BookmarksScreen.css";

/**
 * Saved bookmarks, newest first. Tapping one opens its detail screen; Android's back button
 * (or "‹ Bookmarks") returns to the list.
 */
export function BookmarksScreen() {
  const [items, setItems] = useState<Bookmark[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openId, setOpenId] = useState<string | null>(null);

  const load = (cursor: string | null) =>
    listBookmarks({ cursor })
      .then((page) => {
        setItems((old) => (cursor && old ? [...old, ...page.items] : page.items));
        setNext(page.next);
      })
      .catch((e) => setError(String(e)));

  useEffect(() => {
    load(null);
  }, []);

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
        }}
      />
    );
  }

  return (
    <main className="screen with-tabs">
      <h1>Bookmarks</h1>
      {error ? (
        <p className="screen-empty">{error}</p>
      ) : items === null ? null : items.length === 0 ? (
        <p className="screen-empty">
          No bookmarks yet. Share a link from any app and pick Save bookmark.
        </p>
      ) : (
        <>
          <ul className="bookmark-list">
            {items.map((bookmark) => (
              <li key={bookmark.id}>
                <button type="button" className="bookmark-item" onClick={() => open(bookmark.id)}>
                  <span className="bookmark-title">{bookmark.title ?? bookmark.url}</span>
                  <span className="bookmark-meta">
                    {hostOf(bookmark.url)} · {formatDate(bookmark.createdAt)}
                  </span>
                  {bookmark.tags.length > 0 && (
                    <span className="bookmark-tags">
                      {bookmark.tags.map((tag) => (
                        <span key={tag.name} className="bookmark-tag">
                          {tag.name}
                        </span>
                      ))}
                    </span>
                  )}
                </button>
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
