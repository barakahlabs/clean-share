import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent,
  type ReactNode,
  type RefObject,
} from "react";
import "./BottomSheet.css";

type Props = {
  /** Called for a tap on the dimmed background, Escape, or swiping the sheet down. */
  onClose: () => void;
  /** Fills the visible height instead of fitting its content; changing it animates the size. */
  tall?: boolean;
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
export function BottomSheet({ onClose, labelledBy, role = "dialog", tall = false, children }: Props) {
  const viewport = useVisibleViewport();
  const sheet = useRef<HTMLDivElement>(null);
  usePageHeldStill();
  const drag = useSwipeDown(sheet, onClose);

  // Growing or shrinking (`tall`): animate from the old height to the new one.
  const lastHeight = useRef<number | null>(null);
  useLayoutEffect(() => {
    const el = sheet.current;
    if (!el) return;
    const height = el.getBoundingClientRect().height;
    const from = lastHeight.current;
    lastHeight.current = height;
    if (from === null || Math.abs(from - height) < 2 || reducedMotion()) return;
    el.animate([{ height: `${from}px` }, { height: `${height}px` }], {
      duration: 220,
      easing: "cubic-bezier(0.2, 0.8, 0.2, 1)",
    });
  }, [tall]);

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
        ref={sheet}
        className={tall ? "sheet tall" : "sheet"}
        role={role}
        aria-modal="true"
        aria-labelledby={labelledBy}
        onClick={(event) => event.stopPropagation()}
      >
        {/* Swipe down here to close (also the sheet's top padding). */}
        <div className="sheet-grab" aria-hidden="true" {...drag}>
          <span />
        </div>
        {children}
      </div>
    </div>
  );
}

/**
 * Keeps the page behind the sheet where it is while the sheet is open. Otherwise iOS scrolls it
 * when a field in the sheet gets focus and the keyboard opens (the bookmark list jumped), and
 * touches that miss the sheet scroll it. The page is pinned at its scroll position and put back
 * when the sheet closes; a sheet over a sheet keeps the outer one's position.
 */
function usePageHeldStill() {
  useLayoutEffect(() => {
    const body = document.body.style;
    if (body.position === "fixed") return;
    const y = window.scrollY;
    const before = { position: body.position, top: body.top, left: body.left, right: body.right };
    Object.assign(body, { position: "fixed", top: `-${y}px`, left: "0", right: "0" });
    return () => {
      Object.assign(body, before);
      window.scrollTo(0, y);
    };
  }, []);
}

function reducedMotion() {
  return window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Dragging the grab handle moves the sheet with the finger; letting go far enough down (or with a
 * quick flick) closes it, otherwise it springs back.
 */
function useSwipeDown(sheet: RefObject<HTMLDivElement | null>, onClose: () => void) {
  const start = useRef<{ y: number; time: number } | null>(null);

  const move = (offset: number) => {
    const el = sheet.current;
    if (!el) return;
    el.style.transform = offset ? `translateY(${offset}px)` : "";
    const backdrop = el.parentElement;
    if (backdrop) backdrop.style.backgroundColor = offset ? `rgba(0, 0, 0, ${0.4 * Math.max(0, 1 - offset / el.offsetHeight)})` : "";
  };

  return {
    onPointerDown: (event: PointerEvent) => {
      start.current = { y: event.clientY, time: event.timeStamp };
      event.currentTarget.setPointerCapture(event.pointerId);
    },
    onPointerMove: (event: PointerEvent) => {
      if (start.current) move(Math.max(0, event.clientY - start.current.y));
    },
    onPointerUp: (event: PointerEvent) => {
      const from = start.current;
      start.current = null;
      const el = sheet.current;
      if (!from || !el) return;
      const offset = Math.max(0, event.clientY - from.y);
      const speed = offset / Math.max(1, event.timeStamp - from.time);
      if (offset > Math.min(120, el.offsetHeight / 3) || (offset > 24 && speed > 0.6)) {
        el.style.transition = "transform 150ms ease-in";
        el.style.transform = `translateY(${el.offsetHeight}px)`;
        setTimeout(onClose, 150);
      } else {
        el.style.transition = "transform 150ms ease-out";
        move(0);
        setTimeout(() => (el.style.transition = ""), 150);
      }
    },
    onPointerCancel: () => {
      start.current = null;
      move(0);
    },
  };
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
