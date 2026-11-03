import { useCallback, useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  deleteBookmark,
  getBookmark,
  onPreview,
  retryPreview,
  thumbnailUrl,
  type Bookmark,
} from "../bookmarks";
import { getSettings } from "../settings";
import { Thumbnail } from "../components/Thumbnail";
import { EmbedPlayer } from "../components/EmbedPlayer";
import { playerFor } from "../embeds";
import { shareText } from "../shareTarget";
import { ConfirmSheet } from "../components/ConfirmSheet";
import { formatDate, hostOf, titleFromLink } from "../format";
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
 * One bookmark: picture and description (with link previews on), or a tap-to-play player for
 * video and audio links (YouTube, Vimeo, Spotify, SoundCloud, X, TikTok), title, cleaned link,
 * site and date, tags and note, with Open, Share, Edit (the save sheet in edit mode) and Delete (after a
 * confirm sheet).
 */
export function BookmarkDetailScreen({ id, onBack, onDeleted }: Props) {
  const [bookmark, setBookmark] = useState<Bookmark | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [previewsOn, setPreviewsOn] = useState(false);

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

  useEffect(() => {
    getSettings()
      .then((s) => setPreviewsOn(s.previews))
      .catch(() => {});
  }, []);

  // Its preview finished in the background: show it.
  useEffect(() => onPreview((done) => done === id && load()), [id, load]);

  const retry = () => {
    if (!bookmark) return;
    setBookmark({ ...bookmark, previewStatus: "pending" });
    retryPreview(bookmark.id).catch((e) => setError(String(e)));
  };

  // The edit sheet has a history entry, so Android's back button closes it (and only it).
  useEffect(() => {
    if (!editing) return;
    history.pushState({ bookmarksPage: "edit" }, "");
    const onBackPressed = () => setEditing(false);
    window.addEventListener("popstate", onBackPressed);
    return () => window.removeEventListener("popstate", onBackPressed);
  }, [editing]);

  const player = bookmark ? playerFor(bookmark) : null;

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
            {player ? (
              <EmbedPlayer
                player={player}
                poster={thumbnailUrl(bookmark)}
                title={bookmark.title ?? titleFromLink(bookmark.url)}
              />
            ) : (
              <Thumbnail src={thumbnailUrl(bookmark)} className="detail-image" />
            )}
            <h1 className="detail-title">{bookmark.title ?? titleFromLink(bookmark.url)}</h1>
            {bookmark.description && <p className="detail-description">{bookmark.description}</p>}
            <p className="detail-url">{bookmark.url}</p>
            <p className="detail-meta">
              {[
                bookmark.siteName ?? hostOf(bookmark.url),
                bookmark.author,
                bookmark.published && `Published ${formatPublished(bookmark.published)}`,
                `Saved ${formatDate(bookmark.createdAt)}`,
              ]
                .filter(Boolean)
                .join(" · ")}
            </p>
            {previewsOn && bookmark.previewStatus === "pending" && (
              <p className="detail-meta">Fetching preview…</p>
            )}
            {previewsOn && bookmark.previewStatus === "failed" && (
              <p className="detail-meta">
                Couldn't load a preview.{" "}
                <button type="button" className="text-button" onClick={retry}>
                  Retry
                </button>
              </p>
            )}
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

/** A publish date from the page (usually ISO 8601) as a local date; as given if it isn't one. */
function formatPublished(value: string) {
  const ms = Date.parse(value);
  return Number.isNaN(ms) ? value : formatDate(ms);
}
