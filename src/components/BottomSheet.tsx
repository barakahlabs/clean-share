import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import "./BottomSheet.css";

type Props = {
  /** Called for a tap on the dimmed background or Escape. */
  onClose: () => void;
  /** Id of the element that names the sheet (its title). */
  labelledBy: string;
  role?: "dialog" | "alertdialog";
  children: ReactNode;
};

/**
 * A sheet that slides up from the bottom over a dimmed page. It stays above the on-screen
 * keyboard: Android shrinks the page for the keyboard, but iOS web views don't, so the sheet
 * follows the visible area (`visualViewport`) and is lifted by the keyboard's height.
 */
export function BottomSheet({ onClose, labelledBy, role = "dialog", children }: Props) {
  const viewport = useVisibleViewport();

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => event.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const style = {
    "--keyboard-inset": `${viewport.keyboard}px`,
    "--visible-height": `${viewport.height}px`,
    // On the screen's bottom edge the sheet clears the home indicator; on the keyboard it only
    // needs a little room.
    "--sheet-pad-bottom": viewport.keyboard ? "12px" : "calc(env(safe-area-inset-bottom) + 20px)",
  } as CSSProperties;

  return (
    <div className="sheet-backdrop" style={style} onClick={onClose}>
      <div
        className="sheet"
        role={role}
        aria-modal="true"
        aria-labelledby={labelledBy}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}

/** Height of the visible area, and how much of the page the keyboard covers (0 when it resizes). */
function useVisibleViewport() {
  const read = () => {
    const vv = window.visualViewport;
    if (!vv) return { height: window.innerHeight, keyboard: 0 };
    return {
      height: vv.height,
      keyboard: Math.max(0, Math.round(window.innerHeight - vv.height - vv.offsetTop)),
    };
  };
  const [viewport, setViewport] = useState(read);

  useEffect(() => {
    const vv = window.visualViewport;
    if (!vv) return;
    const update = () => setViewport(read());
    vv.addEventListener("resize", update);
    vv.addEventListener("scroll", update);
    return () => {
      vv.removeEventListener("resize", update);
      vv.removeEventListener("scroll", update);
    };
  }, []);

  return viewport;
}
