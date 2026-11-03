import { useState } from "react";
import { exportBookmarks, importBookmarks, type ExportFormat, type ImportReport } from "../transfer";
import "./Screen.css";
import "./SettingsScreen.css";

const SOURCES: Record<ImportReport["format"], string> = {
  cleanshare: "a Clean Share backup",
  html: "a bookmarks file",
  pocket: "a Pocket export",
  raindrop: "a Raindrop export",
  csv: "a CSV file",
};

/**
 * Settings → Export and import. Bookmarks only live on this phone, so a backup is the way to
 * keep a copy, move to a new phone, or take them to a browser. Export opens the share sheet;
 * import reads a file picked with the system picker.
 */
export function TransferScreen({ onBack }: { onBack: () => void }) {
  const [busy, setBusy] = useState<"export" | "import" | null>(null);
  const [report, setReport] = useState<ImportReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  const doExport = (format: ExportFormat) => {
    setBusy("export");
    setError(null);
    exportBookmarks(format)
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(null));
  };

  const doImport = () => {
    setBusy("import");
    setError(null);
    setReport(null);
    importBookmarks()
      .then((r) => r && setReport(r))
      .catch((e) => setError(String(e)))
      .finally(() => setBusy(null));
  };

  return (
    <main className="screen with-tabs">
      <header className="screen-header">
        <button type="button" className="text-button back-button" onClick={onBack}>
          ‹ Settings
        </button>
      </header>
      <h1>Export and import</h1>

      <section className="settings-section">
        <ul className="settings-list">
          <li>
            <button type="button" className="settings-row" disabled={busy !== null} onClick={() => doExport("json")}>
              <span>Back up bookmarks</span>
              <span className="settings-value">JSON</span>
            </button>
          </li>
          <li>
            <button type="button" className="settings-row" disabled={busy !== null} onClick={() => doExport("html")}>
              <span>Export for a browser</span>
              <span className="settings-value">HTML</span>
            </button>
          </li>
        </ul>
        <p className="settings-note">
          Bookmarks are only kept on this phone. A backup has everything (tags, notes, dates); save
          it somewhere safe, like Files or Drive, to restore it here or on a new phone. The browser
          file works with Chrome, Firefox, Safari and most bookmark apps.
        </p>
      </section>

      <section className="settings-section">
        <ul className="settings-list">
          <li>
            <button type="button" className="settings-row" disabled={busy !== null} onClick={doImport}>
              <span>{busy === "import" ? "Importing…" : "Import a file"}</span>
              <span className="settings-value" aria-hidden="true">
                ›
              </span>
            </button>
          </li>
        </ul>
        <p className="settings-note">
          A Clean Share backup, a browser's bookmarks file, or a Pocket or Raindrop export. Links are
          cleaned as they come in; ones you already have get the new tags and keep the rest.
        </p>
        {report && (
          <p className="settings-note transfer-report" role="status">
            From {SOURCES[report.format]}: {count(report.added, "new bookmark")}
            {report.merged > 0 && `, ${report.merged} already saved`}
            {report.skipped > 0 && `, ${report.skipped} skipped (not a web link)`}.
          </p>
        )}
        {error && <p className="settings-note settings-error">{error}</p>}
      </section>
    </main>
  );
}

function count(n: number, noun: string) {
  return `${n} ${noun}${n === 1 ? "" : "s"}`;
}
