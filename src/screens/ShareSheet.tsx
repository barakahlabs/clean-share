import { useEffect, useState } from "react";
import { cleanText, hasTracking, type Analysis } from "../linkCleaner";
import { shareText } from "../shareTarget";
import { BottomSheet } from "../components/BottomSheet";
import { CleanedText } from "../components/CleanedText";
import { CopyButton } from "../components/CopyButton";
import { useSaveState } from "../hooks/useSaveState";
import shareIcon from "../assets/share.svg";
import { BookmarkForm } from "./BookmarkForm";
import "./SharedTextScreen.css";
import "./ShareSheet.css";

type Props = {
  text: string;
  /** `save` opens straight on the bookmark form (another app's "Save bookmark", the Clean tab). */
  start: "clean" | "save";
  /** Called when the sheet closes, `saved` if a bookmark was saved. */
  onClose: (saved: boolean) => void;
};

/**
 * What another app shared, in a bottom sheet over the app: the text with its tracking struck
 * through, Copy and Share (both use the clean text), and "Save as bookmark", which grows the same
 * sheet into the bookmark form. Copy and Share leave it open; swiping down, the background or ×
 * closes it (an unsaved bookmark is dropped); saving closes it after "Saved".
 */
export function ShareSheet({ text, start, onClose }: Props) {
  const [step, setStep] = useState(start);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const { state: save } = useSaveState(text);

  useEffect(() => {
    let current = true;
    cleanText(text)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    return () => {
      current = false;
    };
  }, [text]);

  const close = () => onClose(false);
  const tracked = analysis !== null && hasTracking(analysis);

  return (
    <BottomSheet labelledBy={step === "save" ? "bookmark-form-title" : "share-sheet-title"} tall={step === "save"} onClose={close}>
      {step === "save" ? (
        <BookmarkForm text={text} onClose={close} onSaved={() => onClose(true)} />
      ) : (
        <div className="share-sheet">
          <h2 id="share-sheet-title" className="visually-hidden">
            Shared with you
          </h2>
          {analysis ? (
            <CleanedText analysis={analysis} label="Shared with you" action="when you copy or share" />
          ) : (
            <p className="screen-empty">Cleaning…</p>
          )}
          <div className="share-sheet-actions">
            <CopyButton text={analysis?.cleaned ?? null} label={tracked ? "Copy clean link" : "Copy"} className="share-button secondary" />
            <button
              type="button"
              className="share-button primary"
              disabled={!analysis}
              onClick={() => analysis && shareText(analysis.cleaned).catch(console.error)}
            >
              <img src={shareIcon} alt="" className="share-icon" />
              Share
            </button>
            {(save === "ready" || save === "saved") && (
              <button type="button" className="share-button secondary wide" onClick={() => setStep("save")}>
                {save === "saved" ? "Edit bookmark" : "Save as bookmark"} <span className="share-chevron">›</span>
              </button>
            )}
          </div>
        </div>
      )}
    </BottomSheet>
  );
}
