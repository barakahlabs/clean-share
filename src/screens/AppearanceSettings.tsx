import { useEffect, useState } from "react";
import { PALETTES, getAppearance, resolvedMode, setAppearance, type Mode } from "../theme/appearance";
import "./AppearanceSettings.css";

const MODES: { id: Mode; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

/**
 * Settings → Appearance: light, dark or the system's, and the colour palette. Changes apply at
 * once. The palette picker is there to choose one; once chosen, it can go.
 */
export function AppearanceSettings() {
  const [appearance, setLocal] = useState(getAppearance);
  const [systemDark, setSystemDark] = useState(() => resolvedMode("system") === "dark");

  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemDark(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);

  // From the saved value, not this render's copy: two quick taps mustn't undo each other.
  const change = (next: Partial<typeof appearance>) => {
    const value = { ...getAppearance(), ...next };
    setLocal(value);
    setAppearance(value);
  };
  const dark = appearance.mode === "dark" || (appearance.mode === "system" && systemDark);

  return (
    <section className="settings-section">
      <h2 className="settings-heading">Appearance</h2>
      <div className="appearance">
        <div className="mode-switch" role="radiogroup" aria-label="Light or dark">
          {MODES.map((mode) => (
            <button
              key={mode.id}
              type="button"
              role="radio"
              aria-checked={appearance.mode === mode.id}
              className={appearance.mode === mode.id ? "mode-option on" : "mode-option"}
              onClick={() => change({ mode: mode.id })}
            >
              {mode.label}
            </button>
          ))}
        </div>

        <div className="palette-grid" role="radiogroup" aria-label="Colour palette">
          {PALETTES.map((palette) => {
            const [primary, soft, background] = dark ? palette.dark : palette.light;
            const on = appearance.palette === palette.id;
            return (
              <button
                key={palette.id}
                type="button"
                role="radio"
                aria-checked={on}
                className={on ? "palette-option on" : "palette-option"}
                onClick={() => change({ palette: palette.id })}
              >
                <span className="palette-swatch" style={{ background }} aria-hidden="true">
                  <span style={{ background: primary }} />
                  <span style={{ background: soft }} />
                </span>
                <span className="palette-name">{palette.name}</span>
              </button>
            );
          })}
        </div>
      </div>
    </section>
  );
}
