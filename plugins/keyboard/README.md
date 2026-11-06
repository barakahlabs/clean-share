# tauri-plugin-keyboard

iOS only: shrinks the web view to the space above the on-screen keyboard (accessory bar included),
animated with the keyboard, the way Android resizes the window. Without it, iOS covers the page
with the keyboard, and the page has to measure it with `visualViewport` (which lags behind the
keyboard's animation, so bottom sheets jumped) and can be dragged around under it.

Nothing to call: `.plugin(tauri_plugin_keyboard::init())` is enough. Other platforms ignore it.
