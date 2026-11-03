/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set by the Tauri CLI during `tauri dev` / `tauri build` (vite.config.ts `envPrefix`). The
   * iOS CI workflow builds the frontend on its own and sets it there too. */
  readonly TAURI_ENV_PLATFORM?: "ios" | "android" | "macos" | "windows" | "linux";
}
