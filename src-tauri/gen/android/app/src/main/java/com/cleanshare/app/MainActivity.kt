package com.cleanshare.app

import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Edge-to-edge (required from Android 15). The keyboard plugin (plugins/keyboard) then keeps
    // the web view above the on-screen keyboard.
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  // No stretch when pulling past the top or bottom: Android 12+ stretches the whole WebView,
  // which dragged the fixed tab bar off the screen. (CSS overscroll-behavior doesn't stop it.)
  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.overScrollMode = View.OVER_SCROLL_NEVER
  }
}
