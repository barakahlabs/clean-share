import SwiftRs
import Tauri
import UIKit
import WebKit

class SetModeArgs: Decodable {
  let mode: String
}

/// Makes the window light or dark, to match the mode chosen in the app. The status bar, the
/// keyboard and the page's `prefers-color-scheme` all follow the window's interface style;
/// "system" hands it back to the phone's setting.
class AppearancePlugin: Plugin {
  private weak var webview: WKWebView?

  override func load(webview: WKWebView) {
    self.webview = webview
  }

  @objc public func setMode(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(SetModeArgs.self)
    let style: UIUserInterfaceStyle
    switch args.mode {
    case "light": style = .light
    case "dark": style = .dark
    default: style = .unspecified
    }
    DispatchQueue.main.async {
      self.webview?.window?.overrideUserInterfaceStyle = style
      invoke.resolve()
    }
  }
}

@_cdecl("init_plugin_appearance")
func initPlugin() -> Plugin {
  return AppearancePlugin()
}
