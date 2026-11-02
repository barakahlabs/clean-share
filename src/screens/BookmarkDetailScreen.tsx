import { useCallback, useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { deleteBookmark, getBookmark, type Bookmark } from "../bookmarks";
import { shareText } from "../shareTarget";
import { ConfirmSheet } from "../components/ConfirmSheet";
import { formatDate, hostOf } from "../format";
import { SaveBookmarkSheet } from "./SaveBookmarkSheet";
import "./Screen.css";
import "./BookmarkDetailScreen.css";

type Props = {
  id: string;
  onBack: () => void;
  /** Called after the bookmark was deleted (the list should reload). */
  onDeleted: () => void;
};

/**
 * One bookmark: title, cleaned link, site and date, tags and note, with Open, Share, Edit (the
 * save sheet in edit mode) and Delete (after a confirm sheet).
 */
export function BookmarkDetailScreen({ id, onBack, onDeleted }: Props) {
  const [bookmark, setBookmark] = useState<Bookmark | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  const load = useCallback(
    () =>
      getBookmark(id)
        .then((found) => (found ? setBookmark(found) : setError("This bookmark no longer exists.")))
        .catch((e) => setError(String(e))),
    [id],
  );

  useEffect(() => {
    load();
  }, [load]);

  // The edit sheet has a history entry, so Android's back button closes it (and only it).
  useEffect(() => {
    if (!editing) return;
    history.pushState({ bookmarksPage: "edit" }, "");
    const onBackPressed = () => setEditing(false);
    window.addEventListener("popstate", onBackPressed);
    return () => window.removeEventListener("popstate", onBackPressed);
  }, [editing]);

  return (
    <main className="screen with-tabs">
      <header className="screen-header">
        <button type="button" className="text-button back-button" onClick={onBack}>
          ‹ Bookmarks
        </button>
      </header>

      {error ? (
        <p className="screen-empty">{error}</p>
      ) : bookmark === null ? null : (
        <>
          <section className="detail-card">
            <h1 className="detail-title">{bookmark.title ?? hostOf(bookmark.url)}</h1>
            <p className="detail-url">{bookmark.url}</p>
            <p className="detail-meta">
              {hostOf(bookmark.url)} · Saved {formatDate(bookmark.createdAt)}
            </p>
            {bookmark.tags.length > 0 && (
              <ul className="detail-tags" aria-label="Tags">
                {bookmark.tags.map((tag) => (
                  <li key={tag.name} className="bookmark-tag">
                    {tag.name}
                  </li>
                ))}
              </ul>
            )}
            {bookmark.note && <p className="detail-note">{bookmark.note}</p>}
          </section>

          <div className="detail-actions">
            <button type="button" className="detail-action primary" onClick={() => openUrl(bookmark.url).catch(console.error)}>
              Open
            </button>
            <button type="button" className="detail-action" onClick={() => shareText(bookmark.url).catch(console.error)}>
              Share
            </button>
            <button type="button" className="detail-action" onClick={() => setEditing(true)}>
              Edit
            </button>
          </div>

          <button type="button" className="text-button danger detail-delete" onClick={() => setConfirmDelete(true)}>
            Delete bookmark
          </button>
        </>
      )}

      {editing && bookmark && (
        <SaveBookmarkSheet
          text={bookmark.url}
          onDone={(saved) => {
            history.back();
            if (saved) load();
          }}
        />
      )}

      {confirmDelete && bookmark && (
        <ConfirmSheet
          title="Delete this bookmark?"
          detail={bookmark.title ?? bookmark.url}
          confirmLabel="Delete bookmark"
          destructive
          onCancel={() => setConfirmDelete(false)}
          onConfirm={() => {
            setConfirmDelete(false);
            deleteBookmark(bookmark.id)
              .then(onDeleted)
              .catch((e) => setError(String(e)));
          }}
        />
      )}
    </main>
  );
}
