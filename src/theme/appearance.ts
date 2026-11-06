// Appearance: which colour palette, and light, dark or the system's choice. Applied as two
// attributes on <html> (data-palette, data-mode), which src/theme/palettes.css turns into the
// colour variables every component uses. Saved on this device only.
//
// index.html applies the saved choice before the first paint (the same logic, inline), so the app
// doesn't flash in the wrong colours on launch.

export type Mode = "system" | "light" | "dark";

export type Appearance = { palette: string; mode: Mode };

/** The palettes in palettes.css, with colours for the picker's swatches. */
export const PALETTES: { id: string; name: string; light: string[]; dark: string[] }[] = [
  { id: "violet", name: "Violet", light: ["#5B2BE0", "#EEE8FD", "#F6F5FB"], dark: ["#8E6CFA", "#2B2350", "#100E18"] },
  { id: "aurora", name: "Aurora", light: ["#5A2BE2", "#ECE7FC", "#F5F5FA"], dark: ["#8F72FF", "#262050", "#0A0914"] },
  { id: "ocean", name: "Ocean & Coral", light: ["#0B63CE", "#E1EDFC", "#F3F7FC"], dark: ["#4C9BFF", "#13305A", "#06101D"] },
  { id: "periwinkle", name: "Periwinkle", light: ["#5865D6", "#EBEDFC", "#F6F7FC"], dark: ["#8B95F2", "#262B4A", "#14161F"] },
  { id: "sage", name: "Sage", light: ["#4E7A5E", "#E7F0E8", "#F6F7F3"], dark: ["#8DC0A0", "#26372B", "#141713"] },
  { id: "sky-sand", name: "Sky & Sand", light: ["#2F7FC1", "#E6F1FA", "#FAF8F4"], dark: ["#6FB2EA", "#1F3346", "#161719"] },
  { id: "rose", name: "Dusty Rose", light: ["#B4566E", "#F8E8EC", "#FBF7F7"], dark: ["#E496A9", "#3D2830", "#181416"] },
  { id: "cobalt", name: "Cobalt & Peach", light: ["#3B6BC9", "#E7EEFB", "#F6F8FB"], dark: ["#7FA6F0", "#1F2F4D", "#12161D"] },
];

const KEY = "appearance";
const DEFAULT: Appearance = { palette: "violet", mode: "system" };
const dark = window.matchMedia("(prefers-color-scheme: dark)");

export function getAppearance(): Appearance {
  try {
    const saved = JSON.parse(localStorage.getItem(KEY) ?? "null");
    const palette = PALETTES.some((p) => p.id === saved?.palette) ? saved.palette : DEFAULT.palette;
    const mode = ["system", "light", "dark"].includes(saved?.mode) ? saved.mode : DEFAULT.mode;
    return { palette, mode };
  } catch {
    return DEFAULT;
  }
}

export function setAppearance(appearance: Appearance) {
  try {
    localStorage.setItem(KEY, JSON.stringify(appearance));
  } catch {
    // Not saved (storage unavailable): it still applies until the app closes.
  }
  apply(appearance);
}

/** Light or dark right now: the chosen mode, or the system's. */
export function resolvedMode(mode: Mode): "light" | "dark" {
  return mode === "system" ? (dark.matches ? "dark" : "light") : mode;
}

function apply({ palette, mode }: Appearance) {
  const root = document.documentElement;
  root.dataset.palette = palette;
  root.dataset.mode = resolvedMode(mode);
}

// Follow the system switching between light and dark while the app is open.
dark.addEventListener("change", () => apply(getAppearance()));
apply(getAppearance());
