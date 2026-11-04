import { useEffect, useState } from "react";
import { shareText } from "../shareTarget";
import { cleanText, hasTracking, type Analysis } from "../linkCleaner";
import { CleanedText } from "../components/CleanedText";
import { CopyButton } from "../components/CopyButton";
import { useSaveState } from "../hooks/useSaveState";
import shareIcon from "../assets/share.svg";
import "./Screen.css";
import "./SharedTextScreen.css";

type Props = {
  text: string;
  onClose: () => void;
  /** Opens the save screen for this text (title, tags, note). */
  onSave: () => void;
};

/**
 * Shows text shared from another app with its tracking highlighted; copies or shares it cleaned.
 * "Save bookmark" opens the save screen; once the link is saved the button edits it instead.
 */
export function SharedTextScreen({ text, onClose, onSave }: Props) {
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const { state: save, error: saveError } = useSaveState(text);

  useEffect(() => {
    let current = true;
    setAnalysis(null);
    cleanText(text)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    return () => {
      current = false;
    };
  }, [text]);

  return (
    <main className="screen">
      <header className="screen-header">
        <h1>Clean Share</h1>
        <button type="button" className="text-button" onClick={onClose}>
          Done
        </button>
      </header>
      {analysis ? (
        <CleanedText analysis={analysis} label="Shared with you" action="when you copy or share" />
      ) : (
        <p className="screen-empty">Cleaning…</p>
      )}

      {saveError && <p className="save-error">{saveError}</p>}

      <footer className="share-footer">
        {save !== "none" && (
          <button
            type="button"
            className="share-button secondary"
            disabled={save !== "ready" && save !== "saved"}
            onClick={onSave}
          >
            {save === "saved" ? "Edit bookmark" : "Save bookmark"}
          </button>
        )}
        <CopyButton text={analysis?.cleaned ?? null} label={analysis && hasTracking(analysis) ? "Copy clean link" : "Copy"} />
        <button
          type="button"
          className="share-button"
          disabled={!analysis}
          onClick={() => analysis && shareText(analysis.cleaned).catch(console.error)}
        >
          <img src={shareIcon} alt="" className="share-icon" />
          {analysis && hasTracking(analysis) ? "Share clean link" : "Share"}
        </button>
      </footer>
    </main>
  );
}
