import { useEffect, useState } from "react";
import {
  fetchPreview,
  listTags,
  prepareBookmark,
  saveBookmark,
  setBookmarkTags,
  suggestTags,
  updateBookmark,
  type LinkPreview,
  type Prepared,
  type Suggestion,
  type TagCount,
  type TagSource,
} from "../bookmarks";
import { BottomSheet } from "../components/BottomSheet";
import { TagPicker } from "../components/TagPicker";
import "./SaveBookmarkSheet.css";

/** How long the title has to stay unchanged before suggestions use it. */
const TITLE_DELAY_MS = 300;

type Props = {
  text: string;
  /** Called after saving (`true`) or cancelling (`false`). */
  onDone: (saved: boolean) => void;
};

/**
 * A bottom sheet that saves the first link in shared text as a bookmark: title, tags, and an
 * optional note. If the link (after cleaning) is already saved, it edits that bookmark instead.
 * The fields scroll; the Save button stays at the bottom of the sheet, above the keyboard.
 *
 * Suggested tags show as chips under the tag field and update as tags are picked. A tag taken
 * from a suggestion is saved with source "suggested".
 *
 * With link previews on, a new link's preview is fetched while the sheet is open: it shows the
 * site, title and description, fills an empty title, and improves the suggestions (page keywords
 * and kind). The background fetcher reuses it after saving. No image here: pictures are only
 * shown from the phone, once the thumbnail is saved.
 */
export function SaveBookmarkSheet({ text, onDone }: Props) {
  const [prepared, setPrepared] = useState<Prepared | null>(null);
  const [allTags, setAllTags] = useState<TagCount[]>([]);
  const [title, setTitle] = useState("");
  const [note, setNote] = useState("");
  const [showNote, setShowNote] = useState(false);
  const [tags, setTags] = useState<string[]>([]);
  // Where each tag came from, by lowercased name; tags not in here are the user's own.
  const [sources, setSources] = useState<Record<string, TagSource>>({});
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [settledTitle, setSettledTitle] = useState("");
  const [titleTouched, setTitleTouched] = useState(false);
  const [preview, setPreview] = useState<LinkPreview | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    Promise.all([prepareBookmark(text), listTags()])
      .then(([prep, existingTags]) => {
        if (!current) return;
        setPrepared(prep);
        setAllTags(existingTags);
        const saved = prep.existing;
        setTitle(saved ? (saved.title ?? "") : (prep.title ?? ""));
        setNote(saved?.note ?? "");
        setShowNote(Boolean(saved?.note));
        setTags(saved ? saved.tags.map((tag) => tag.name) : []);
        setSources(
          Object.fromEntries((saved?.tags ?? []).map((tag) => [tag.name.toLowerCase(), tag.source])),
        );
      })
      .catch((e) => current && setError(String(e)));
    return () => {
      current = false;
    };
  }, [text]);

  const link = prepared?.links[0];
  const existing = prepared?.existing ?? null;

  // New links only: a saved bookmark already has its preview (or a Retry on its screen).
  useEffect(() => {
    if (!link || existing) return;
    let current = true;
    setPreviewLoading(true);
    fetchPreview(text)
      .then((found) => current && setPreview(found))
      .catch(() => {})
      .finally(() => current && setPreviewLoading(false));
    return () => {
      current = false;
    };
  }, [link, existing, text]);

  // The page's title, unless one came with the share or the user typed one.
  useEffect(() => {
    if (preview?.title && !titleTouched && !title.trim()) setTitle(preview.title);
    // Runs only when the preview arrives, not on every keystroke.
  }, [preview]);

  useEffect(() => {
    const timer = setTimeout(() => setSettledTitle(title.trim()), TITLE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [title]);

  // Ask again whenever the picked tags change: they bring related tags along.
  useEffect(() => {
    if (!link) return;
    let current = true;
    suggestTags({ text, title: settledTitle || null, picked: tags })
      .then((found) => current && setSuggestions(found))
      .catch(() => current && setSuggestions([]));
    return () => {
      current = false;
    };
    // `preview`: once it arrives, the backend has the page's keywords and kind.
  }, [link, text, settledTitle, tags, preview]);

  // Tags added by typing or from the dropdown are the user's own.
  const changeTags = (next: string[]) => {
    const added = next.filter((name) => !tags.includes(name));
    setSources((old) => ({ ...old, ...Object.fromEntries(added.map((n) => [n.toLowerCase(), "user"])) }));
    setTags(next);
  };

  const acceptSuggestion = (name: string) => {
    setSuggestions((old) => old.filter((s) => s.name !== name));
    setSources((old) => ({ ...old, [name.toLowerCase()]: "suggested" }));
    setTags((old) => [...old, name]);
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const tagInputs = tags.map((name) => ({ name, source: sources[name.toLowerCase()] ?? "user" }));
      if (existing) {
        await updateBookmark(existing.id, { title, note });
        await setBookmarkTags(existing.id, tagInputs);
      } else {
        await saveBookmark({ sharedText: text, title, note, tags: tagInputs });
      }
      onDone(true);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <BottomSheet labelledBy="save-sheet-title" onClose={() => onDone(false)}>
      <header className="save-header">
        <h2 id="save-sheet-title" className="sheet-title">
          {existing ? "Edit bookmark" : "Save bookmark"}
        </h2>
        <button type="button" className="text-button" onClick={() => onDone(false)}>
          Cancel
        </button>
      </header>

      {prepared && !link ? (
        <p className="save-hint">There's no link in what was shared, so there's nothing to save.</p>
      ) : (
        <form
          className="save-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          <div className="save-scroll">
            {link && (
              <p className="save-link" title={link.cleaned}>
                {link.cleaned}
              </p>
            )}
            {existing && <p className="save-hint">Already saved. Changes update this bookmark.</p>}
            {previewLoading && !preview && <p className="save-hint">Loading preview…</p>}
            {preview && (preview.siteName || preview.description) && (
              <div className="save-preview">
                {preview.siteName && <span className="save-preview-site">{preview.siteName}</span>}
                {preview.description && <span className="save-preview-text">{preview.description}</span>}
              </div>
            )}

            <label className="save-field">
              <span className="save-label">Title</span>
              <input
                className="save-input"
                value={title}
                placeholder={link?.cleaned ?? "Title"}
                enterKeyHint="done"
                onChange={(event) => {
                  setTitleTouched(true);
                  setTitle(event.target.value);
                }}
              />
            </label>

            <div className="save-field">
              <span className="save-label">Tags</span>
              <TagPicker selected={tags} available={allTags} onChange={changeTags} />
              {suggestions.length > 0 && (
                <div className="tag-suggested" role="group" aria-label="Suggested tags">
                  {suggestions.map((suggestion) => (
                    <button
                      key={suggestion.name}
                      type="button"
                      className="tag-suggested-chip"
                      aria-label={`Add suggested tag ${suggestion.name}`}
                      onClick={() => acceptSuggestion(suggestion.name)}
                    >
                      + {suggestion.name}
                    </button>
                  ))}
                </div>
              )}
            </div>

            {showNote ? (
              <label className="save-field">
                <span className="save-label">Note</span>
                <textarea
                  className="save-input save-note"
                  value={note}
                  rows={3}
                  autoFocus={!existing}
                  placeholder="Why you saved it"
                  onChange={(event) => setNote(event.target.value)}
                />
              </label>
            ) : (
              <button type="button" className="text-button add-note" onClick={() => setShowNote(true)}>
                + Add note
              </button>
            )}

            {error && <p className="save-error">{error}</p>}
          </div>

          <button type="submit" className="save-button" disabled={!prepared || saving}>
            {saving ? "Saving…" : existing ? "Save changes" : "Save bookmark"}
          </button>
        </form>
      )}
    </BottomSheet>
  );
}
