import { useState } from "react";
import { getMode, setMode, type Mode } from "../theme/appearance";
import "./AppearanceSettings.css";

const MODES: { id: Mode; label: string }[] = [
  { id: "system", label: "System" },
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
];

/** Settings → Appearance: light, dark or the system's. Applies at once. */
export function AppearanceSettings() {
  const [mode, setLocal] = useState(getMode);

  const change = (next: Mode) => {
    setLocal(next);
    setMode(next);
  };

  return (
    <section className="settings-section">
      <h2 className="settings-heading">Appearance</h2>
      <div className="appearance">
        <div className="mode-switch" role="radiogroup" aria-label="Light or dark">
          {MODES.map((option) => (
            <button
              key={option.id}
              type="button"
              role="radio"
              aria-checked={mode === option.id}
              className={mode === option.id ? "mode-option on" : "mode-option"}
              onClick={() => change(option.id)}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
