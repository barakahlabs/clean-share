import "./TabBar.css";

export type Tab = "bookmarks" | "settings";

const TABS: { id: Tab; label: string; icon: string }[] = [
  // Bookmark ribbon and a gear, as simple strokes so they follow the text color.
  { id: "bookmarks", label: "Bookmarks", icon: "M6 3h12v18l-6-4-6 4z" },
  {
    id: "settings",
    label: "Settings",
    icon: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm7.4-3a7.4 7.4 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14.5 3h-5l-.4 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6a7.4 7.4 0 0 0 0 2.4l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2l.4 2.6h5l.4-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z",
  },
];

export function TabBar({ current, onChange }: { current: Tab; onChange: (tab: Tab) => void }) {
  return (
    <nav className="tab-bar" aria-label="Sections">
      {TABS.map((tab) => (
        <button
          key={tab.id}
          type="button"
          className={tab.id === current ? "tab active" : "tab"}
          aria-current={tab.id === current ? "page" : undefined}
          onClick={() => onChange(tab.id)}
        >
          <svg viewBox="0 0 24 24" className="tab-icon" aria-hidden="true">
            <path d={tab.icon} />
          </svg>
          {tab.label}
        </button>
      ))}
    </nav>
  );
}
