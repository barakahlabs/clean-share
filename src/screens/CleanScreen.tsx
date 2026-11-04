import { useEffect, useState } from "react";
import { shareText } from "../shareTarget";
import { cleanText, type Analysis } from "../linkCleaner";
import { pasteText } from "../clipboard";
import { CleanedText } from "../components/CleanedText";
import { CopyButton } from "../components/CopyButton";
import { useSaveState } from "../hooks/useSaveState";
import shareIcon from "../assets/share.svg";
import "./Screen.css";
import "./SharedTextScreen.css";
import "./CleanScreen.css";

/** How long typing has to pause before cleaning. */
const CLEAN_DELAY_MS = 150;

type Props = {
  /** Opens the save sheet for this text. */
  onSave: (text: string) => void;
  /** Bumped after a save, so "Save bookmark" turns into "Edit bookmark". */
  saves: number;
};

/**
 * Clean tab: paste or type a link (or any text with links), see its tracking highlighted as you
 * type, then copy, share or save it, for links that didn't come through the share sheet.
 */
export function CleanScreen({ onSave, saves }: Props) {
  const [text, setText] = useState("");
  const [settled, setSettled] = useState("");
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const { state: save } = useSaveState(settled, saves);

  useEffect(() => {
    const timer = setTimeout(() => setSettled(text.trim()), CLEAN_DELAY_MS);
    return () => clearTimeout(timer);
  }, [text]);

  useEffect(() => {
    if (!settled) {
      setAnalysis(null);
      return;
    }
    let current = true;
    cleanText(settled)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    return () => {
      current = false;
    };
  }, [settled]);

  const paste = () =>
    pasteText().then((pasted) => {
      if (pasted.trim()) setText(pasted);
    });

  const ready = analysis !== null && settled !== "";

  return (
    <main className="screen with-tabs">
      <h1>Clean a link</h1>

      <section className="clean-input">
        <textarea
          className="clean-field"
          value={text}
          rows={3}
          placeholder="Paste or type a link"
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          aria-label="Link to clean"
          onChange={(event) => setText(event.target.value)}
        />
        <div className="clean-actions">
          <button type="button" className="text-button" onClick={paste}>
            Paste
          </button>
          {text && (
            <button type="button" className="text-button" onClick={() => setText("")}>
              Clear
            </button>
          )}
        </div>
      </section>

      {ready && <CleanedText analysis={analysis} label="Cleaned" action="when you copy or share" />}

      {ready && (
        <footer className="share-footer">
          {save !== "none" && (
            <button
              type="button"
              className="share-button secondary"
              disabled={save !== "ready" && save !== "saved"}
              onClick={() => onSave(settled)}
            >
              {save === "saved" ? "Edit bookmark" : "Save bookmark"}
            </button>
          )}
          <CopyButton text={analysis.cleaned} label={analysis.trackingCount ? "Copy clean link" : "Copy"} />
          <button type="button" className="share-button" onClick={() => shareText(analysis.cleaned).catch(console.error)}>
            <img src={shareIcon} alt="" className="share-icon" />
            {analysis.trackingCount ? "Share clean link" : "Share"}
          </button>
        </footer>
      )}
    </main>
  );
}
