# tauri-plugin-share-target

Receives text shared from other apps (links count as text), and opens the system
share sheet to send text onwards.

- **Android**: adds an `ACTION_SEND` `text/plain` intent-filter to `MainActivity` at build time
  (see `build.rs`); `android/.../ShareTargetPlugin.kt` reads the intent.
- **iOS**: the app's Share Extension (`src-tauri/gen/apple/ShareExtension`) saves the text to the
  `group.com.sirajulm.cleanshare` App Group and opens the app via `cleanshare://share`;
  `ios/Sources/ShareTargetPlugin.swift` reads it.
- **Sharing out**: `share_text` opens Android's share chooser (excluding this app) or iOS's
  `UIActivityViewController`.
- **Desktop**: `get_shared` always returns no text; `share_text` returns an error.

## JS usage

Each share also says which share-sheet entry was picked: `intent` is `"share"` (Clean & Share)
or `"save"` (Save bookmark). Android reads it from the component the share arrived on (an
`activity-alias` ending in `.SaveBookmark` is "save"); iOS from the `sharedIntent` App Group key
that each Share Extension writes. Anything else counts as `"share"`.

```ts
invoke("plugin:share-target|get_shared")          // { text?: string, intent?: "share" | "save" }, clears it
addPluginListener("share-target", "shared", ...)  // fired when a new share arrives
invoke("plugin:share-target|share_text", { text }) // opens the share sheet
invoke("plugin:share-target|return_to_source")    // back to the sharing app (Android); false on iOS
```

See `src/shareTarget.ts` in the app for a wrapper.
