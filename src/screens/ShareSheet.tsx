import { useEffect, useRef, useState } from "react";
import { cleanText, hasTracking, type Analysis } from "../linkCleaner";
import { shareText } from "../shareTarget";
import { pasteText } from "../clipboard";
import { BottomSheet } from "../components/BottomSheet";
import { CleanedText } from "../components/CleanedText";
import { CopyButton } from "../components/CopyButton";
import { useSaveState } from "../hooks/useSaveState";
import shareIcon from "../assets/share.svg";
import { BookmarkForm } from "./BookmarkForm";
import "./SharedTextScreen.css";
import "./ShareSheet.css";

/** How long typing has to pause before cleaning. */
const CLEAN_DELAY_MS = 150;

type Props = {
  /** What was shared (empty when typing one in). */
  text: string;
  /**
   * `clean`: show what was shared, cleaned. `input`: an empty field to paste or type into (the
   * Clean button). `save`: straight on the bookmark form (another app's "Save bookmark").
   */
  start: "clean" | "input" | "save";
  /** Called when the sheet closes, `saved` if a bookmark was saved. */
  onClose: (saved: boolean) => void;
};

/**
 * What another app shared, in a bottom sheet over the app: the text with its tracking struck
 * through, Copy and Share (both use the clean text), and "Save as bookmark", which grows the same
 * sheet into the bookmark form. Copy and Share leave it open; swiping down, the background or ×
 * closes it (an unsaved bookmark is dropped); saving closes it after "Saved".
 *
 * Opened from the Clean button, the first step starts with a field to paste or type into, and
 * the text is cleaned as you type.
 */
export function ShareSheet({ text, start, onClose }: Props) {
  const typing = start === "input";
  const [step, setStep] = useState<"clean" | "save">(start === "save" ? "save" : "clean");
  const [input, setInput] = useState("");
  const [settled, setSettled] = useState("");
  const source = typing ? settled : text;
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const { state: save } = useSaveState(source);

  useEffect(() => {
    if (!typing) return;
    const timer = setTimeout(() => setSettled(input.trim()), CLEAN_DELAY_MS);
    return () => clearTimeout(timer);
  }, [typing, input]);

  useEffect(() => {
    if (!source.trim()) {
      setAnalysis(null);
      return;
    }
    let current = true;
    cleanText(source)
      .then((result) => current && setAnalysis(result))
      .catch(console.error);
    return () => {
      current = false;
    };
  }, [source]);

  // Focus the field without scrolling anything into view: the sheet already sits on the keyboard.
  const field = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    if (typing) field.current?.focus({ preventScroll: true });
  }, [typing]);

  const paste = () =>
    pasteText().then((pasted) => {
      if (pasted.trim()) setInput(pasted);
    });

  const close = () => onClose(false);
  const tracked = analysis !== null && hasTracking(analysis);

  return (
    <BottomSheet labelledBy={step === "save" ? "bookmark-form-title" : "share-sheet-title"} tall={step === "save"} onClose={close}>
      {step === "save" ? (
        <BookmarkForm text={source} onClose={close} onSaved={() => onClose(true)} />
      ) : (
        <div className="share-sheet">
          {typing ? (
            <>
              <h2 id="share-sheet-title" className="sheet-title">
                Clean a link
              </h2>
              <div className="clean-input">
                <textarea
                  ref={field}
                  className="clean-field"
                  value={input}
                  rows={2}
                  placeholder="Paste or type a link"
                  autoCapitalize="none"
                  autoCorrect="off"
                  spellCheck={false}
                  aria-label="Link to clean"
                  onChange={(event) => setInput(event.target.value)}
                />
                <div className="clean-actions">
                  <button type="button" className="text-button" onClick={paste}>
                    Paste
                  </button>
                  {input && (
                    <button type="button" className="text-button" onClick={() => setInput("")}>
                      Clear
                    </button>
                  )}
                </div>
              </div>
              {analysis && source && <CleanedText analysis={analysis} label="Cleaned" action="when you copy or share" />}
            </>
          ) : (
            <>
              <h2 id="share-sheet-title" className="visually-hidden">
                Shared with you
              </h2>
              {analysis ? (
                <CleanedText analysis={analysis} label="Shared with you" action="when you copy or share" />
              ) : (
                <p className="screen-empty">Cleaning…</p>
              )}
            </>
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
