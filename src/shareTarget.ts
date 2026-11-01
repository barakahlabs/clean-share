import { addPluginListener, invoke } from "@tauri-apps/api/core";

/** Which share-sheet entry the user picked: "Clean & Share" or "Save bookmark". */
export type ShareIntent = "share" | "save";

/** Something another app shared with us. */
export type Shared = { text: string; intent: ShareIntent };

type SharedContent = { text?: string | null; intent?: ShareIntent | null };

/** Returns what another app shared and clears it, or null if nothing is pending. */
export async function getShared(): Promise<Shared | null> {
  const shared = await invoke<SharedContent>("plugin:share-target|get_shared");
  return shared.text ? { text: shared.text, intent: shared.intent ?? "share" } : null;
}

/**
 * Calls `onShare` with what another app shared: once for a share that
 * launched the app, then again for each share while it's running.
 * Returns a cleanup function.
 */
export function onShared(onShare: (shared: Shared) => void): () => void {
  let unregister: (() => void) | undefined;
  let stopped = false;

  const deliver = () =>
    getShared()
      .then((shared) => shared && onShare(shared))
      .catch(console.error);

  deliver();

  addPluginListener("share-target", "shared", deliver)
    .then((listener) => {
      if (stopped) listener.unregister();
      else unregister = () => listener.unregister();
    })
    // Desktop has no native share plugin to listen to.
    .catch(() => {});

  return () => {
    stopped = true;
    unregister?.();
  };
}

/** Opens the system share sheet so the user can send `text` to another app. */
export function shareText(text: string): Promise<void> {
  return invoke("plugin:share-target|share_text", { text });
}
