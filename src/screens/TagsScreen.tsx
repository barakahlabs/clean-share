import { useEffect, useState } from "react";
import { deleteTag, listTags, renameTag, type TagCount } from "../bookmarks";
import "./Screen.css";
import "./TagsScreen.css";

/** What a row is doing: showing its actions, being renamed, or asking to confirm a delete. */
type Mode = { kind: "idle" } | { kind: "rename"; name: string; draft: string } | { kind: "delete"; name: string };

/**
 * Settings → Tags: every tag with how many bookmarks use it. Unused tags (0) stay until deleted
 * here. Rename merges into an existing tag of that name; delete removes the tag from every
 * bookmark that has it, after a confirm that says how many.
 */
export function TagsScreen({ onBack }: { onBack: () => void }) {
  const [tags, setTags] = useState<TagCount[] | null>(null);
  const [mode, setMode] = useState<Mode>({ kind: "idle" });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = () =>
    listTags()
      .then(setTags)
      .catch((e) => setError(String(e)));

  useEffect(() => {
    reload();
  }, []);

  const run = (action: Promise<void>) => {
    setBusy(true);
    setError(null);
    action
      .then(() => {
        setMode({ kind: "idle" });
        return reload();
      })
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(false));
  };

  const existing = (name: string) =>
    tags?.find((tag) => tag.name.toLowerCase() === name.trim().toLowerCase());

  return (
    <main className="screen with-tabs">
      <header className="screen-header">
        <button type="button" className="text-button back-button" onClick={onBack}>
          ‹ Settings
        </button>
      </header>
      <h1>Tags</h1>
      {error && <p className="tags-error">{error}</p>}

      {tags === null ? null : tags.length === 0 ? (
        <p className="screen-empty">No tags yet. Tags you add to bookmarks show up here.</p>
      ) : (
        <ul className="tag-list">
          {tags.map((tag) => (
            <li key={tag.name} className="tag-row">
              {mode.kind === "rename" && mode.name === tag.name ? (
                <form
                  className="tag-edit"
                  onSubmit={(event) => {
                    event.preventDefault();
                    const to = mode.draft.trim();
                    if (!to || to === tag.name) return setMode({ kind: "idle" });
                    run(renameTag(tag.name, to));
                  }}
                >
                  <input
                    className="tag-input"
                    value={mode.draft}
                    autoFocus
                    aria-label={`New name for ${tag.name}`}
                    onChange={(event) => setMode({ ...mode, draft: event.target.value })}
                  />
                  {(() => {
                    const target = existing(mode.draft);
                    return target && target.name !== tag.name ? (
                      <p className="tag-note">
                        "{target.name}" already exists: the two will be merged.
                      </p>
                    ) : null;
                  })()}
                  <div className="tag-actions">
                    <button type="button" className="text-button" onClick={() => setMode({ kind: "idle" })}>
                      Cancel
                    </button>
                    <button type="submit" className="text-button" disabled={busy || !mode.draft.trim()}>
                      Save
                    </button>
                  </div>
                </form>
              ) : mode.kind === "delete" && mode.name === tag.name ? (
                <div className="tag-confirm">
                  <p className="tag-note">
                    {tag.count === 0
                      ? `Delete "${tag.name}"? No bookmarks use it.`
                      : `Remove "${tag.name}" from ${bookmarkCount(tag.count)} and delete it?`}
                  </p>
                  <div className="tag-actions">
                    <button type="button" className="text-button" onClick={() => setMode({ kind: "idle" })}>
                      Cancel
                    </button>
                    <button
                      type="button"
                      className="text-button danger"
                      disabled={busy}
                      onClick={() => run(deleteTag(tag.name))}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ) : (
                <div className="tag-line">
                  <span className="tag-name">{tag.name}</span>
                  <span className="tag-count">{tag.count === 0 ? "Unused" : bookmarkCount(tag.count)}</span>
                  <span className="tag-actions">
                    <button
                      type="button"
                      className="text-button"
                      onClick={() => setMode({ kind: "rename", name: tag.name, draft: tag.name })}
                    >
                      Rename
                    </button>
                    <button
                      type="button"
                      className="text-button danger"
                      onClick={() => setMode({ kind: "delete", name: tag.name })}
                    >
                      Delete
                    </button>
                  </span>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </main>
  );
}

function bookmarkCount(count: number) {
  return count === 1 ? "1 bookmark" : `${count} bookmarks`;
}
