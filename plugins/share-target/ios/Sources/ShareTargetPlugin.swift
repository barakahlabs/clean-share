import SwiftRs
import Tauri
import UIKit
import WebKit

class ShareTextArgs: Decodable {
  let text: String
}

/// Reads text that the app's Share Extensions left in the shared App Group.
///
/// An extension can't hand data to the app directly, so it writes to
/// App Group UserDefaults and opens the app. When the app becomes active
/// with something pending, a `shared` event tells JS to call `get_shared`.
///
/// Each extension also writes which share-sheet entry it is ("share" for
/// Clean & Share, "save" for Save bookmark); older shares without it count as "share".
class ShareTargetPlugin: Plugin {
  // Must match the App Group and keys used by the Share Extensions.
  private let appGroup = "group.com.sirajulm.cleanshare"
  private let sharedTextKey = "sharedText"
  private let sharedIntentKey = "sharedIntent"

  private var becameActiveObserver: NSObjectProtocol?
  private weak var webview: WKWebView?

  private var defaults: UserDefaults? { UserDefaults(suiteName: appGroup) }

  override func load(webview: WKWebView) {
    self.webview = webview
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

  /// Opens the iOS share sheet so the user can send text to another app.
  @objc public func shareText(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(ShareTextArgs.self)
    // Share links as URLs so the share sheet offers link-specific actions.
    let item: Any = URL(string: args.text).flatMap { ["http", "https"].contains($0.scheme) ? $0 : nil } ?? args.text

    DispatchQueue.main.async { [weak self] in
      guard let self else { return }
      guard var viewController = self.webview?.window?.rootViewController else {
        invoke.reject("No view controller to present the share sheet from")
        return
      }
      // Present on top of anything already showing.
      while let presented = viewController.presentedViewController { viewController = presented }
      let sheet = UIActivityViewController(activityItems: [item], applicationActivities: nil)
      // iPad shows the sheet as a popover, which needs an anchor: the bottom-center button.
      if let popover = sheet.popoverPresentationController {
        let bounds = viewController.view.bounds
        popover.sourceView = viewController.view
        popover.sourceRect = CGRect(x: bounds.midX, y: bounds.maxY - 60, width: 0, height: 0)
      }
      viewController.present(sheet, animated: true)
      invoke.resolve()
    }
  }

  /// iOS has no API to go back to the app that shared; the user switches back themselves.
  @objc public func returnToSource(_ invoke: Invoke) {
    invoke.resolve(["returned": false])
  }

  @objc public func getShared(_ invoke: Invoke) {
    guard let text = defaults?.string(forKey: sharedTextKey) else {
      invoke.resolve([:])
      return
    }
    let intent = defaults?.string(forKey: sharedIntentKey) == "save" ? "save" : "share"
    defaults?.removeObject(forKey: sharedTextKey)
    defaults?.removeObject(forKey: sharedIntentKey)
    invoke.resolve(["text": text, "intent": intent])
  }
}

@_cdecl("init_plugin_share_target")
public func initPlugin() -> Plugin {
  return ShareTargetPlugin()
}
