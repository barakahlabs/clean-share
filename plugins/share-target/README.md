# tauri-plugin-share-target

Receives text shared from other apps (links count as text), and opens the system
share sheet to send text onwards.

- **Android**: adds an `ACTION_SEND` `text/plain` intent-filter to `MainActivity` at build time
  (see `build.rs`); `android/.../ShareTargetPlugin.kt` reads the intent.
- **iOS**: the app's Share Extension (`src-tauri/gen/apple/ShareExtension`) saves the text to the
  `group.com.cleanshare.app` App Group and opens the app via `cleanshare://share`;
  `ios/Sources/ShareTargetPlugin.swift` reads it.
- **Sharing out**: `share_text` opens Android's share chooser (excluding this app) or iOS's
  `UIActivityViewController`.
- **Desktop**: `get_shared` always returns no text; `share_text` returns an error.

## JS usage

```ts
invoke("plugin:share-target|get_shared")          // { text?: string }, clears it
addPluginListener("share-target", "shared", ...)  // fired when a new share arrives
invoke("plugin:share-target|share_text", { text }) // opens the share sheet
```

See `src/shareTarget.ts` in the app for a wrapper.
