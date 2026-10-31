import UIKit
import UniformTypeIdentifiers

/// Share Extension entry point: shows no UI of its own.
///
/// It grabs the shared URL or text, stores it in the App Group that the main
/// app's share-target plugin reads, then opens the main app.
final class ShareViewController: UIViewController {
  // Must match ShareTargetPlugin.swift in plugins/share-target.
  private let appGroup = "group.com.sirajulm.cleanshare"
  private let sharedTextKey = "sharedText"
  private let appURL = URL(string: "cleanshare://share")!

  override func viewDidLoad() {
    super.viewDidLoad()
    Task { await handleShare() }
  }

  @MainActor
  private func handleShare() async {
    if let text = await loadSharedText(), !text.isEmpty {
      UserDefaults(suiteName: appGroup)?.set(text, forKey: sharedTextKey)
      openMainApp()
    }
    extensionContext?.completeRequest(returningItems: nil)
  }

  /// Prefers a URL (e.g. Safari's share button), falls back to plain text.
  private func loadSharedText() async -> String? {
    let items = extensionContext?.inputItems as? [NSExtensionItem] ?? []
    let providers = items.flatMap { $0.attachments ?? [] }

    for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.url.identifier) {
      if let url = try? await provider.loadItem(forTypeIdentifier: UTType.url.identifier) as? URL {
        return url.absoluteString
      }
    }
    for provider in providers where provider.hasItemConformingToTypeIdentifier(UTType.plainText.identifier) {
      if let text = try? await provider.loadItem(forTypeIdentifier: UTType.plainText.identifier) as? String {
        return text.trimmingCharacters(in: .whitespacesAndNewlines)
      }
    }
    return nil
  }

  /// Share extensions have no API to open their app, so find UIApplication in the
  /// responder chain and call `open(_:options:completionHandler:)` through the
  /// Objective-C runtime (it's compile-time unavailable in extensions).
  /// If this ever stops working, the text is still saved and shows up next time the app opens.
  private func openMainApp() {
    typealias OpenURL = @convention(c) (AnyObject, Selector, NSURL, NSDictionary, AnyObject?) -> Void
    let selector = NSSelectorFromString("openURL:options:completionHandler:")

    var responder: UIResponder? = self
    while let current = responder {
      if current is UIApplication, current.responds(to: selector) {
        let open = unsafeBitCast(current.method(for: selector), to: OpenURL.self)
        open(current, selector, appURL as NSURL, NSDictionary(), nil)
        return
      }
      responder = current.next
    }
  }
}
