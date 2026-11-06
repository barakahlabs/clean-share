# tauri-plugin-appearance

Makes the native parts of the app follow the light or dark mode chosen in the app, on Android and
iOS. The page's own colours are CSS; this covers what CSS can't reach. Without it, forcing Dark
on a phone set to light (or the other way round) left the status bar's clock and icons in the
phone's colours, unreadable on the app's background.

| Platform | What the plugin does |
|---|---|
| iOS | `AppearancePlugin.swift`: sets the window's interface style (`overrideUserInterfaceStyle`). The status bar, the keyboard and the page's `prefers-color-scheme` follow it. |
| Android | `AppearancePlugin.kt`: sets the status and navigation bar icons dark or light (the app is edge-to-edge, so the bars themselves are see-through). |

One command: `invoke("plugin:appearance|set_mode", { mode })` with `"system"`, `"light"` or
`"dark"`. The app calls it whenever it applies a mode (src/theme/appearance.ts), including when
the phone switches between light and dark. Desktop ignores it.
