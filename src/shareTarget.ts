import { addPluginListener, invoke } from "@tauri-apps/api/core";

type SharedContent = { text?: string | null };

/** Returns text shared from another app and clears it, or null if nothing is pending. */
export async function getSharedText(): Promise<string | null> {
  const shared = await invoke<SharedContent>("plugin:share-target|get_shared");
  return shared.text ?? null;
}

/**
 * Calls `onText` with text shared from another app: once for a share that
 * launched the app, then again for each share while it's running.
 * Returns a cleanup function.
 */
export function onSharedText(onText: (text: string) => void): () => void {
  let unregister: (() => void) | undefined;
  let stopped = false;

  const deliver = () =>
    getSharedText()
      .then((text) => text && onText(text))
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
