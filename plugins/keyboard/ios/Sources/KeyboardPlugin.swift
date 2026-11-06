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
///
/// WebKit still scrolls the web view to show a focused field, even past the end of the page. Inside
/// a fixed layer (a bottom sheet over a held-still page) the field stays put but its cursor moves
/// with that scroll, so it's drawn below the field. While the keyboard is up, the web view isn't
/// allowed past the end of its content; the page scrolling normally is untouched.
class KeyboardPlugin: Plugin {
  private weak var webview: WKWebView?
  private var observers: [NSObjectProtocol] = []
  private var offset: NSKeyValueObservation?
  private var keyboardUp = false

  override func load(webview: WKWebView) {
    self.webview = webview
    offset = webview.scrollView.observe(\.contentOffset) { [weak self] scrollView, _ in
      self?.keepInside(scrollView)
    }
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

  /// Moves the web view back inside its content if WebKit scrolled it past the end.
  private func keepInside(_ scrollView: UIScrollView) {
    // The user dragging (and bouncing) is left alone.
    guard keyboardUp, !scrollView.isDragging, !scrollView.isDecelerating else { return }
    let inset = scrollView.adjustedContentInset
    let maxX = max(-inset.left, scrollView.contentSize.width + inset.right - scrollView.bounds.width)
    let maxY = max(-inset.top, scrollView.contentSize.height + inset.bottom - scrollView.bounds.height)
    let now = scrollView.contentOffset
    let inside = CGPoint(x: min(max(now.x, -inset.left), maxX), y: min(max(now.y, -inset.top), maxY))
    if inside != now {
      scrollView.contentOffset = inside
    }
  }

  private func follow(_ note: Notification, hiding: Bool) {
    guard let webview, let container = webview.superview, let window = webview.window else { return }
    let info = note.userInfo ?? [:]
    let end = (info[UIResponder.keyboardFrameEndUserInfoKey] as? NSValue)?.cgRectValue ?? .zero
    // How much of the container the keyboard will cover (0 when hidden or floating elsewhere).
    let keyboard = container.convert(window.convert(end, from: nil), from: window)
    let covered = hiding ? 0 : max(0, container.bounds.maxY - keyboard.minY)
    keyboardUp = covered > 0

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
    } completion: { _ in
      self.keepInside(webview.scrollView)
    }
  }
}

@_cdecl("init_plugin_keyboard")
func initPlugin() -> Plugin {
  return KeyboardPlugin()
}
