import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listBookmarks, type Bookmark } from "../bookmarks";
import "./Screen.css";
import "./BookmarksScreen.css";

/** Saved bookmarks, newest first. Tapping one opens its cleaned link. */
export function BookmarksScreen() {
  const [items, setItems] = useState<Bookmark[] | null>(null);
  const [next, setNext] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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

  return (
    <main className="screen with-tabs">
      <h1>Bookmarks</h1>
      {error ? (
        <p className="screen-empty">{error}</p>
      ) : items === null ? null : items.length === 0 ? (
        <p className="screen-empty">
          No bookmarks yet. Share a link from any app, pick Clean Share, and tap Save bookmark.
        </p>
      ) : (
        <>
          <ul className="bookmark-list">
            {items.map((bookmark) => (
              <li key={bookmark.id}>
                <button
                  type="button"
                  className="bookmark-item"
                  onClick={() => openUrl(bookmark.url).catch(console.error)}
                >
                  <span className="bookmark-title">{bookmark.title ?? bookmark.url}</span>
                  <span className="bookmark-meta">
                    {hostOf(bookmark.url)} · {new Date(bookmark.createdAt).toLocaleDateString()}
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

function hostOf(url: string) {
  try {
    return new URL(url).host.replace(/^www\./, "");
  } catch {
    return url;
  }
}
