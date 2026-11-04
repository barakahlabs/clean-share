import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// The tracking rules in use (src-tauri/src/rules.rs): built in, or newer ones downloaded.

export type RulesStatus = {
  revision: string;
  /** True if they were downloaded (newer than the ones built into the app). */
  downloaded: boolean;
  /** When the app last checked for new rules, ms since the epoch. */
  checkedAt: number | null;
};

export function rulesStatus(): Promise<RulesStatus> {
  return invoke("rules_status");
}

/** Checks for new rules now; returns the status afterwards. */
export function checkRules(): Promise<RulesStatus> {
  return invoke("rules_check");
}

/** Calls `onChange` after each background check. */
export function onRulesStatus(onChange: (status: RulesStatus) => void): () => void {
  const stop = listen<RulesStatus>("rules-status", (event) => onChange(event.payload));
  return () => {
    stop.then((unlisten) => unlisten()).catch(() => {});
  };
}
