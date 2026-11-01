import { useEffect, useRef } from "react";
import "./ConfirmSheet.css";

type Props = {
  title: string;
  detail?: string;
  confirmLabel: string;
  /** Red confirm button, for actions that can't be undone. */
  destructive?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
};

/**
 * A bottom sheet that asks to confirm an action. Built in the page (not `window.confirm`, which
 * Android's WebView doesn't reliably show), so it looks the same on both platforms.
 * Cancel, a tap on the dimmed background, Escape, or Android's back button all cancel.
 *
 * Mount it only while it's open: it adds a history entry so the back button closes it.
 */
export function ConfirmSheet({ title, detail, confirmLabel, destructive, onConfirm, onCancel }: Props) {
  const confirmed = useRef(false);
  // Latest callbacks, so the history listener doesn't go stale.
  const handlers = useRef({ onConfirm, onCancel });
  handlers.current = { onConfirm, onCancel };

  useEffect(() => {
    history.pushState({ sheet: true }, "");
    const onPop = () => (confirmed.current ? handlers.current.onConfirm() : handlers.current.onCancel());
    const onKey = (event: KeyboardEvent) => event.key === "Escape" && history.back();
    window.addEventListener("popstate", onPop);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("popstate", onPop);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  // Every way out goes through history.back(), so the sheet's history entry is always removed.
  const close = (confirm: boolean) => {
    confirmed.current = confirm;
    history.back();
  };

  return (
    <div className="sheet-backdrop" onClick={() => close(false)}>
      <div
        className="sheet"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="sheet-title"
        onClick={(event) => event.stopPropagation()}
      >
        <h2 id="sheet-title" className="sheet-title">
          {title}
        </h2>
        {detail && <p className="sheet-detail">{detail}</p>}
        <button
          type="button"
          className={destructive ? "sheet-button destructive" : "sheet-button"}
          onClick={() => close(true)}
        >
          {confirmLabel}
        </button>
        <button type="button" className="sheet-button" onClick={() => close(false)} autoFocus>
          Cancel
        </button>
      </div>
    </div>
  );
}
