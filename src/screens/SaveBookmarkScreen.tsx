import { useEffect, useState } from "react";
import {
  listTags,
  prepareBookmark,
  saveBookmark,
  setBookmarkTags,
  updateBookmark,
  type Prepared,
  type TagCount,
} from "../bookmarks";
import { TagPicker } from "../components/TagPicker";
import "./Screen.css";
import "./SaveBookmarkScreen.css";

/**
 * Saves the first link in shared text as a bookmark, with a title, tags and a note. If that link
 * (after cleaning) is already saved, the same screen edits the existing bookmark instead.
 * `onDone` is called after saving or cancelling.
 */
export function SaveBookmarkScreen({ text, onDone }: { text: string; onDone: () => void }) {
  const [prepared, setPrepared] = useState<Prepared | null>(null);
  const [allTags, setAllTags] = useState<TagCount[]>([]);
  const [title, setTitle] = useState("");
  const [note, setNote] = useState("");
  const [tags, setTags] = useState<string[]>([]);
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
        setTags(saved ? saved.tags.map((tag) => tag.name) : []);
      })
      .catch((e) => current && setError(String(e)));
    return () => {
      current = false;
    };
  }, [text]);

  const link = prepared?.links[0];
  const existing = prepared?.existing ?? null;

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      const tagInputs = tags.map((name) => ({ name }));
      if (existing) {
        await updateBookmark(existing.id, { title, note });
        await setBookmarkTags(existing.id, tagInputs);
      } else {
        await saveBookmark({ sharedText: text, title, note, tags: tagInputs });
      }
      onDone();
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  return (
    <main className="screen">
      <header className="screen-header">
        <h1>{existing ? "Edit bookmark" : "Save bookmark"}</h1>
        <button type="button" className="text-button" onClick={onDone}>
          Cancel
        </button>
      </header>

      {prepared && !link ? (
        <p className="screen-empty">There's no link in what was shared, so there's nothing to save.</p>
      ) : (
        <form
          className="save-form"
          onSubmit={(event) => {
            event.preventDefault();
            save();
          }}
        >
          {link && (
            <p className="save-link" title={link.cleaned}>
              {link.cleaned}
            </p>
          )}
          {existing && <p className="save-hint">Already saved. Changes update the existing bookmark.</p>}

          <label className="save-field">
            <span className="save-label">Title</span>
            <input
              className="save-input"
              value={title}
              placeholder={link?.cleaned ?? "Title"}
              onChange={(event) => setTitle(event.target.value)}
            />
          </label>

          <div className="save-field">
            <span className="save-label">Tags</span>
            <TagPicker selected={tags} available={allTags} onChange={setTags} />
          </div>

          <label className="save-field">
            <span className="save-label">Note</span>
            <textarea
              className="save-input save-note"
              value={note}
              rows={3}
              placeholder="Why you saved it (optional)"
              onChange={(event) => setNote(event.target.value)}
            />
          </label>

          {error && <p className="save-error">{error}</p>}

          <button type="submit" className="save-button" disabled={!prepared || saving}>
            {saving ? "Saving…" : existing ? "Save changes" : "Save bookmark"}
          </button>
        </form>
      )}
    </main>
  );
}
