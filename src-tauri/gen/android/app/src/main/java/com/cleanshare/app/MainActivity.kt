package com.cleanshare.app

import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Edge-to-edge apps aren't resized for the on-screen keyboard. Pad the content by the
    // keyboard's height instead, so the WebView shrinks and the page (e.g. a bottom sheet with a
    // text field) stays above the keyboard. With the keyboard closed the padding is 0.
    val content = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      view.updatePadding(bottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom)
      insets
    }
  }
}
