# tauri-plugin-keyboard

Shrinks the web view to the space above the on-screen keyboard, animated with it, on Android and
iOS, so a page (e.g. a bottom sheet with a text field) lays out above the keyboard with plain CSS.
Like Capacitor's "native" keyboard resize.

| Platform | Without it | What the plugin does |
|---|---|---|
| iOS | The keyboard (and its ✓ accessory bar) covers the web view; pages measure it with `visualViewport`, which lags behind the animation, and the visible area can be dragged around under it. | `KeyboardPlugin.swift`: on keyboard frame changes, resizes the web view's frame with the keyboard's animation. |
| Android | The app is edge-to-edge (required from Android 15), and edge-to-edge windows aren't resized for the keyboard (`adjustResize` does nothing). | `KeyboardPlugin.kt`: pads the activity's content by the keyboard's height (window insets). |

Both are a `KeyboardPlugin` that sets itself up when the web view loads. Nothing to call:
`.plugin(tauri_plugin_keyboard::init())` is enough. Desktop ignores it.
