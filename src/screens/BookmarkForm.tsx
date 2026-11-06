import { useEffect, useRef, useState, type ReactNode } from "react";
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
import { cleanText, hasTracking, type Analysis } from "../linkCleaner";
import { copyText } from "../clipboard";
import { shareText } from "../shareTarget";
import { CleanedText } from "../components/CleanedText";
import { TagPicker } from "../components/TagPicker";
import "./SaveBookmarkSheet.css";

/** How long typing has to pause before the title or link is used. */
const SETTLE_MS = 300;
const LINK_SETTLE_MS = 400;
/** How long "Saved" shows before the sheet closes. */
const SAVED_MS = 900;

type Props = {
  /** What was shared (or the bookmark's link, when editing one). */
  text: string;
  /** False when editing a saved bookmark: changing its link would save a second bookmark. */
  editableLink?: boolean;
  onClose: () => void;
  /** After saving, once "Saved" has shown. */
  onSaved: () => void;
};

/**
 * The bookmark form: link, preview, title, tags and note. Nothing is saved until Save; then the
 * button turns into "Saved" for a moment and `onSaved` closes the sheet.
 *
 * The link field shows the link as shared and can be edited. It's cleaned as you type ("1 removed",
 * tap it to see what and why) and the bookmark gets the cleaned link. After a pause, the form
 * follows the new link: whether it's saved already, its preview, suggested tags, and its title
 * (unless the title was typed). Copy and Share use the cleaned link.
 *
 * Suggested tags show as chips under the tag field and update as tags are picked. A tag taken
 * from a suggestion is saved with source "suggested". With link previews on, a new link's preview
 * fills an empty title and improves the suggestions; the background fetcher reuses it.
 */
export function BookmarkForm({ text, editableLink = true, onClose, onSaved }: Props) {
  // The link field and the text everything else uses: what was shared, until the link is edited.
  const [linkInput, setLinkInput] = useState<string | null>(null);
  const sharedLink = useRef<string | null>(null);
  const [draft, setDraft] = useState(text);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [showWhy, setShowWhy] = useState(false);

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
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const loaded = useRef(false);

  useEffect(() => {
    listTags().then(setAllTags).catch(() => {});
  }, []);

  // The edited link, once typing pauses. Unedited, the whole shared text is used (its title too).
  useEffect(() => {
    if (linkInput === null) return;
    const edited = linkInput.trim() !== sharedLink.current;
    const timer = setTimeout(() => setDraft(edited ? linkInput.trim() : text), edited ? LINK_SETTLE_MS : 0);
    return () => clearTimeout(timer);
  }, [linkInput, text]);

  useEffect(() => {
    let current = true;
    prepareBookmark(draft)
      .then((prep) => {
        if (!current) return;
        setPrepared(prep);
        setPreview(null);
        const saved = prep.existing;
        if (!loaded.current) {
          // First load: the field shows the link as shared, the form what's known about it.
          loaded.current = true;
          sharedLink.current = prep.links[0]?.original ?? "";
          setLinkInput(sharedLink.current);
          setTitle(saved ? (saved.title ?? "") : (prep.title ?? ""));
          setNote(saved?.note ?? "");
          setShowNote(Boolean(saved?.note));
          setTags(saved ? saved.tags.map((tag) => tag.name) : []);
          setSources(Object.fromEntries((saved?.tags ?? []).map((tag) => [tag.name.toLowerCase(), tag.source])));
        } else if (!titleTouched) {
          // Another link: its title, unless the user typed one. Tags and note stay.
          setTitle(saved?.title ?? prep.title ?? "");
        }
      })
      .catch((e) => current && setError(String(e)));
    return () => {
      current = false;
    };
    // `titleTouched` is read, not followed: typing a title doesn't re-check the link.
  }, [draft]);

  // What cleaning removes from the link in the field (for "1 removed", Copy and Share).
  useEffect(() => {
    const link = (linkInput ?? "").trim();
    if (!link) {
      setAnalysis(null);
      return;
    }
    let current = true;
    const timer = setTimeout(
      () =>
        cleanText(link)
          .then((result) => current && setAnalysis(result))
          .catch(() => {}),
      150,
    );
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [linkInput]);

  const link = prepared?.links[0];
  const existing = prepared?.existing ?? null;
  const cleaned = link?.cleaned ?? null;
  // The field's link differs from the one the form shows until typing settles.
  const settling = linkInput !== null && linkInput.trim() !== (draft === text ? sharedLink.current : draft);

  // New links only: a saved bookmark already has its preview (or a Retry on its screen).
  useEffect(() => {
    if (!link || existing) return;
    let current = true;
    setPreviewLoading(true);
    fetchPreview(draft)
      .then((found) => current && setPreview(found))
      .catch(() => {})
      .finally(() => current && setPreviewLoading(false));
    return () => {
      // Another link (or a saved one) now: this one's preview isn't loading any more.
      current = false;
      setPreviewLoading(false);
    };
  }, [link?.cleaned, existing, draft]);

  // The page's title, unless one came with the share or the user typed one.
  useEffect(() => {
    if (preview?.title && !titleTouched && !title.trim()) setTitle(preview.title);
    // Runs only when the preview arrives, not on every keystroke.
  }, [preview]);

  useEffect(() => {
    const timer = setTimeout(() => setSettledTitle(title.trim()), SETTLE_MS);
    return () => clearTimeout(timer);
  }, [title]);

  // Ask again whenever the picked tags change: they bring related tags along.
  useEffect(() => {
    if (!link) return;
    let current = true;
    suggestTags({ text: draft, title: settledTitle || null, picked: tags })
      .then((found) => current && setSuggestions(found))
      .catch(() => current && setSuggestions([]));
    return () => {
      current = false;
    };
    // `preview`: once it arrives, the backend has the page's keywords and kind.
  }, [link?.cleaned, draft, settledTitle, tags, preview]);

  useEffect(() => {
    if (!saved) return;
    const timer = setTimeout(onSaved, SAVED_MS);
    return () => clearTimeout(timer);
  }, [saved, onSaved]);

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
        await saveBookmark({ sharedText: draft, title, note, tags: tagInputs });
      }
      setSaved(true);
    } catch (e) {
      setError(String(e));
      setSaving(false);
    }
  };

  const removed = analysis && hasTracking(analysis) ? analysis.trackingCount + analysis.redirectCount : 0;
  const ready = Boolean(prepared && link && !settling);

  return (
    <form
      className="save-form"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready && !saving) save();
      }}
    >
      <header className="save-header">
        <span aria-hidden="true" />
        <h2 id="bookmark-form-title" className="sheet-title">
          {existing ? "Edit bookmark" : "New bookmark"}
        </h2>
        <button type="button" className="sheet-close" aria-label="Close" onClick={onClose}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden="true">
            <path d="M6 6l12 12M18 6L6 18" />
          </svg>
        </button>
      </header>

      <div className="save-scroll">
        <div className="save-field">
          <label className="save-label" htmlFor="bookmark-link">
            Link
          </label>
          <div className="link-row">
            <input
              id="bookmark-link"
              className="save-input link-input"
              value={linkInput ?? ""}
              readOnly={!editableLink || saved}
              placeholder="Paste or type a link"
              inputMode="url"
              autoCapitalize="none"
              autoCorrect="off"
              spellCheck={false}
              enterKeyHint="done"
              onChange={(event) => setLinkInput(event.target.value)}
            />
            <IconButton label="Copy clean link" confirms disabled={!analysis} onClick={() => analysis && copyText(analysis.cleaned)}>
              <rect x="9" y="9" width="11" height="11" rx="2" />
              <path d="M5 15V5a2 2 0 0 1 2-2h8" />
            </IconButton>
            <IconButton label="Share clean link" disabled={!analysis} onClick={() => analysis && shareText(analysis.cleaned)}>
              <path d="M12 3v12M7 8l5-5 5 5" />
              <path d="M5 13v6a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-6" />
            </IconButton>
          </div>
          {removed > 0 && (
            <button type="button" className="link-removed" aria-expanded={showWhy} onClick={() => setShowWhy((open) => !open)}>
              {removed} removed when saved, copied or shared {showWhy ? "⌃" : "⌄"}
            </button>
          )}
          {showWhy && analysis && removed > 0 && (
            <CleanedText analysis={analysis} label="As shared" action="when you save, copy or share" />
          )}
          {prepared && !link && !settling && <p className="save-hint">Enter a link to save.</p>}
          {existing && <p className="save-hint">Already saved. Changes update this bookmark.</p>}
        </div>

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
            placeholder={cleaned ?? "Title"}
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
              autoFocus
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

      <button
        type="submit"
        className={saved ? "save-button saved" : "save-button"}
        disabled={!ready || (saving && !saved)}
        aria-live="polite"
      >
        {saved ? (
          <>
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" aria-hidden="true">
              <path d="M5 12.5l4.5 4.5L19 7" />
            </svg>
            Saved
          </>
        ) : saving ? (
          "Saving…"
        ) : existing ? (
          "Save changes"
        ) : (
          "Save bookmark"
        )}
      </button>
    </form>
  );
}

/** A round icon-only button that shows a check for a moment after it's used. */
function IconButton({
  label,
  confirms = false,
  disabled,
  onClick,
  children,
}: {
  label: string;
  /** Shows a check for a moment after it's done (Copy; Share has the system sheet). */
  confirms?: boolean;
  disabled: boolean;
  onClick: () => Promise<unknown> | undefined | null | false;
  children: ReactNode;
}) {
  const [done, setDone] = useState(false);
  useEffect(() => {
    if (!done) return;
    const timer = setTimeout(() => setDone(false), 1200);
    return () => clearTimeout(timer);
  }, [done]);

  return (
    <button
      type="button"
      className="icon-button"
      aria-label={label}
      disabled={disabled}
      onClick={() => {
        const result = onClick();
        if (result) result.then(() => confirms && setDone(true)).catch(console.error);
      }}
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
        {done ? <path d="M5 12.5l4.5 4.5L19 7" /> : children}
      </svg>
    </button>
  );
}
