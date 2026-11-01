import { useId, useRef, useState } from "react";
import type { TagCount } from "../bookmarks";
import "./TagPicker.css";

type Props = {
  /** Tags on the bookmark, in the order they were added. */
  selected: string[];
  /** Every existing tag, most used first (from `listTags`). */
  available: TagCount[];
  onChange: (tags: string[]) => void;
};

/** How many existing tags the dropdown shows at once. */
const MAX_SUGGESTIONS = 8;

/**
 * Picks tags for a bookmark: chips for the chosen ones, and a field that suggests existing tags
 * (most used first, filtered as you type) or adds a new one. Enter or a comma adds what's typed;
 * Backspace in an empty field removes the last chip.
 */
export function TagPicker({ selected, available, onChange }: Props) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const listId = useId();

  const typed = normalize(query);
  const isSelected = (name: string) => selected.some((tag) => same(tag, name));

  const suggestions = available
    .filter((tag) => !isSelected(tag.name))
    .filter((tag) => !typed || tag.name.toLowerCase().includes(typed.toLowerCase()))
    .slice(0, MAX_SUGGESTIONS);
  const exact = available.find((tag) => same(tag.name, typed));
  const canAddNew = typed !== "" && !exact && !isSelected(typed);

  const add = (name: string) => {
    const tag = normalize(name);
    if (tag && !isSelected(tag)) onChange([...selected, tag]);
    setQuery("");
    input.current?.focus();
  };

  const remove = (name: string) => onChange(selected.filter((tag) => tag !== name));

  return (
    <div className="tag-picker">
      {selected.length > 0 && (
        <ul className="tag-chips" aria-label="Tags on this bookmark">
          {selected.map((tag) => (
            <li key={tag} className="tag-chip">
              {tag}
              <button type="button" className="tag-chip-remove" aria-label={`Remove ${tag}`} onClick={() => remove(tag)}>
                ×
              </button>
            </li>
          ))}
        </ul>
      )}

      <div className="tag-field">
        <input
          ref={input}
          className="tag-field-input"
          value={query}
          placeholder={selected.length ? "Add another tag" : "Add tags"}
          enterKeyHint="done"
          autoCapitalize="none"
          autoCorrect="off"
          role="combobox"
          aria-expanded={open}
          aria-controls={listId}
          onFocus={() => {
            setOpen(true);
            scrollIntoViewWhenKeyboardOpens(input.current);
          }}
          onBlur={() => setOpen(false)}
          onChange={(event) => {
            const value = event.target.value;
            // A comma ends a tag, like Enter.
            if (value.includes(",")) {
              value.split(",").slice(0, -1).forEach(add);
              setQuery(value.split(",").pop() ?? "");
            } else {
              setQuery(value);
            }
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              if (typed) add(exact?.name ?? typed);
            } else if (event.key === "Backspace" && query === "" && selected.length) {
              remove(selected[selected.length - 1]);
            }
          }}
        />

        {open && (suggestions.length > 0 || canAddNew) && (
          // onMouseDown keeps focus in the field, so the list doesn't close before the tap lands.
          <ul id={listId} className="tag-suggestions" role="listbox" onMouseDown={(e) => e.preventDefault()}>
            {canAddNew && (
              <li role="option" aria-selected={false}>
                <button type="button" className="tag-suggestion add" onClick={() => add(typed)}>
                  Add “{typed}”
                </button>
              </li>
            )}
            {suggestions.map((tag) => (
              <li key={tag.name} role="option" aria-selected={false}>
                <button type="button" className="tag-suggestion" onClick={() => add(tag.name)}>
                  <span>{tag.name}</span>
                  <span className="tag-suggestion-count">{tag.count}</span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/**
 * Brings the field (and the suggestions under it) to the top of its scrolling container, in one
 * instant jump once the keyboard has finished opening, so nothing moves under a finger afterwards.
 * Android resizes the page for the keyboard (several resize events while it animates); iOS
 * changes the visual viewport instead. We wait until the resizing settles, then scroll.
 */
function scrollIntoViewWhenKeyboardOpens(field: HTMLElement | null) {
  if (!field) return;
  const viewport = window.visualViewport;
  let timer = 0;
  const finish = () => {
    viewport?.removeEventListener("resize", settle);
    window.removeEventListener("resize", settle);
    field.scrollIntoView({ block: "start" });
  };
  function settle() {
    window.clearTimeout(timer);
    timer = window.setTimeout(finish, SETTLE_MS);
  }
  viewport?.addEventListener("resize", settle);
  window.addEventListener("resize", settle);
  // If the keyboard is already open (moving from another field) no resize comes at all.
  timer = window.setTimeout(finish, NO_RESIZE_MS);
}

/** Quiet time after the last resize event before the keyboard counts as open. */
const SETTLE_MS = 120;
/** How long to wait for a first resize before scrolling anyway. */
const NO_RESIZE_MS = 450;

/** Same cleanup as the store: trim, drop a leading #, collapse inner spaces. */
function normalize(name: string) {
  return name.trim().replace(/^#+/, "").trim().split(/\s+/).filter(Boolean).join(" ");
}

/** Tag names are case-insensitive. */
function same(a: string, b: string) {
  return a.toLowerCase() === b.toLowerCase();
}
