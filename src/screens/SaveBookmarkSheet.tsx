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
import { BottomSheet } from "../components/BottomSheet";
import { TagPicker } from "../components/TagPicker";
import "./SaveBookmarkSheet.css";

type Props = {
  text: string;
  /** Called after saving (`true`) or cancelling (`false`). */
  onDone: (saved: boolean) => void;
};

/**
 * A bottom sheet that saves the first link in shared text as a bookmark: title, tags, and an
 * optional note. If the link (after cleaning) is already saved, it edits that bookmark instead.
 * The fields scroll; the Save button stays at the bottom of the sheet, above the keyboard.
 */
export function SaveBookmarkSheet({ text, onDone }: Props) {
  const [prepared, setPrepared] = useState<Prepared | null>(null);
  const [allTags, setAllTags] = useState<TagCount[]>([]);
  const [title, setTitle] = useState("");
  const [note, setNote] = useState("");
  const [showNote, setShowNote] = useState(false);
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
        setShowNote(Boolean(saved?.note));
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

            <label className="save-field">
              <span className="save-label">Title</span>
              <input
                className="save-input"
                value={title}
                placeholder={link?.cleaned ?? "Title"}
                enterKeyHint="done"
                onChange={(event) => setTitle(event.target.value)}
              />
            </label>

            <div className="save-field">
              <span className="save-label">Tags</span>
              <TagPicker selected={tags} available={allTags} onChange={setTags} />
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
