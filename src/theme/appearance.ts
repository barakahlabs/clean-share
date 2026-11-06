// Appearance: light, dark or the system's choice. Applied as an attribute on <html> (data-mode),
// which src/theme/colors.css turns into the colour variables every component uses. Saved on this
// device only.
//
// index.html applies the saved choice before the first paint (the same logic, inline), so the app
// doesn't flash in the wrong colours on launch.

export type Mode = "system" | "light" | "dark";

const KEY = "appearance";
const dark = window.matchMedia("(prefers-color-scheme: dark)");

export function getMode(): Mode {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    return saved?.mode === "light" || saved?.mode === "dark" ? saved.mode : "system";
  } catch {
    return "system";
  }
}

export function setMode(mode: Mode) {
  try {
    localStorage.setItem(KEY, JSON.stringify({ mode }));
  } catch {
    // Not saved (storage unavailable): it still applies until the app closes.
  }
  apply(mode);
}

function apply(mode: Mode) {
  document.documentElement.dataset.mode = mode === "system" ? (dark.matches ? "dark" : "light") : mode;
}

// Follow the system switching between light and dark while the app is open.
dark.addEventListener("change", () => apply(getMode()));
apply(getMode());
