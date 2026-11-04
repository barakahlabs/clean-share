import { readText, writeText } from "@tauri-apps/plugin-clipboard-manager";

// The system clipboard, through Tauri's plugin: the WebView's own clipboard API isn't reliable on
// phones. On iOS, reading shows the system's "Allow Paste" prompt.

export function copyText(text: string): Promise<void> {
  return writeText(text);
}

/** The clipboard's text, or "" if it holds none (or reading isn't allowed). */
export async function pasteText(): Promise<string> {
  try {
    return (await readText()) ?? "";
  } catch {
    return "";
  }
}
