import SwiftRs
import Tauri
import UIKit
import WebKit

/// Shrinks the web view to the space above the on-screen keyboard, and back when it closes. The
/// page then never sits under the keyboard: CSS `bottom: 0` is the top of the keyboard, as on
/// Android (KeyboardPlugin.kt does the same there).
///
/// iOS covers the web view with the keyboard (and its accessory bar), so on keyboard frame changes
/// the web view's frame is resized, with the keyboard's own animation. Nothing is left for WebKit
/// to scroll into view or for the user to drag around.
class KeyboardPlugin: Plugin {
  private weak var webview: WKWebView?
  private var observers: [NSObjectProtocol] = []

  override func load(webview: WKWebView) {
    self.webview = webview
    let center = NotificationCenter.default
    // "Will change frame" covers showing, hiding, the accessory bar appearing and a different
    // keyboard (emoji, another language) with another height.
    for name in [UIResponder.keyboardWillChangeFrameNotification, UIResponder.keyboardWillHideNotification] {
      observers.append(
        center.addObserver(forName: name, object: nil, queue: .main) { [weak self] note in
          self?.follow(note, hiding: name == UIResponder.keyboardWillHideNotification)
        })
    }
  }

  deinit {
    observers.forEach(NotificationCenter.default.removeObserver)
  }

  private func follow(_ note: Notification, hiding: Bool) {
    guard let webview, let container = webview.superview, let window = webview.window else { return }
    let info = note.userInfo ?? [:]
    let end = (info[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue)?.cgRectValue ?? .zero
    // How much of the container the keyboard will cover (0 when hidden or floating elsewhere).
    let keyboard = container.convert(window.convert(end, from: nil), from: window)
    let covered = hiding ? 0 : max(0, container.bounds.maxY - keyboard.minY)

    var frame = container.bounds
    frame.size.height -= covered
    guard frame != webview.frame else { return }

    let duration = (info[UIResponder.keyboardAnimationDurationUserInfoKey] as? Double) ?? 0.25
    let curve = (info[UIResponder.keyboardAnimationCurveUserInfoKey] as? UInt) ?? 7
    UIView.animate(
      withDuration: duration,
      delay: 0,
      options: [UIView.AnimationOptions(rawValue: curve << 16), .beginFromCurrentState]
    ) {
      webview.frame = frame
    }
  }
}

@_cdecl("init_plugin_keyboard")
func initPlugin() -> Plugin {
  return KeyboardPlugin()
}
