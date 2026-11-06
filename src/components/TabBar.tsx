import "./TabBar.css";

export type Tab = "bookmarks" | "settings";

// Bookmark ribbon, chain links and a gear, as simple strokes so they follow the text color.
const BOOKMARKS = { id: "bookmarks" as const, label: "Bookmarks", icon: "M6 3h12v18l-6-4-6 4z" };
// Two chain links: paste or type a link to clean.
const CLEAN_ICON =
  "M10 14a4 4 0 0 0 5.66 0l3-3a4 4 0 0 0-5.66-5.66l-1.1 1.1M14 10a4 4 0 0 0-5.66 0l-3 3a4 4 0 0 0 5.66 5.66l1.1-1.1";
const SETTINGS = {
    id: "settings" as const,
    label: "Settings",
    icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z",
};

type Props = {
  current: Tab;
  onChange: (tab: Tab) => void;
  /** The middle button: opens the sheet to paste or type a link (not a page of its own). */
  onClean: () => void;
};

/** Bookmarks and Settings tabs, with the Clean button between them. */
export function TabBar({ current, onChange, onClean }: Props) {
  const tab = (item: typeof BOOKMARKS | typeof SETTINGS) => (
    <button
      key={item.id}
      type="button"
      className={item.id === current ? "tab active" : "tab"}
      aria-current={item.id === current ? "page" : undefined}
      onClick={() => onChange(item.id)}
    >
      <Icon d={item.icon} />
      {item.label}
    </button>
  );

  return (
    <nav className="tab-bar" aria-label="Sections">
      {tab(BOOKMARKS)}
      <button type="button" className="tab" aria-haspopup="dialog" onClick={onClean}>
        <Icon d={CLEAN_ICON} />
        Clean
      </button>
      {tab(SETTINGS)}
    </nav>
  );
}

function Icon({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 24 24" className="tab-icon" aria-hidden="true">
      <path d={d} />
    </svg>
  );
}
