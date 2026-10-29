import SwiftRs
import Tauri
import UIKit
import WebKit

/// Reads text that the app's Share Extension left in the shared App Group.
///
/// The extension can't hand data to the app directly, so it writes to
/// App Group UserDefaults and opens the app. When the app becomes active
/// with something pending, a `shared` event tells JS to call `get_shared`.
class ShareTargetPlugin: Plugin {
  // Must match the App Group and key used by the Share Extension.
  private let appGroup = "group.com.cleanshare.app"
  private let sharedTextKey = "sharedText"

  private var becameActiveObserver: NSObjectProtocol?

  private var defaults: UserDefaults? { UserDefaults(suiteName: appGroup) }

  override func load(webview: WKWebView) {
    becameActiveObserver = NotificationCenter.default.addObserver(
      forName: UIApplication.didBecomeActiveNotification,
      object: nil,
      queue: .main
    ) { [weak self] _ in
      guard let self, self.defaults?.string(forKey: self.sharedTextKey) != nil else { return }
      self.trigger("shared", data: [:])
    }
  }

  deinit {
    if let becameActiveObserver {
      NotificationCenter.default.removeObserver(becameActiveObserver)
    }
  }

  @objc public func getShared(_ invoke: Invoke) {
    guard let text = defaults?.string(forKey: sharedTextKey) else {
      invoke.resolve([:])
      return
    }
    defaults?.removeObject(forKey: sharedTextKey)
    invoke.resolve(["text": text])
  }
}

@_cdecl("init_plugin_share_target")
public func initPlugin() -> Plugin {
  return ShareTargetPlugin()
}
