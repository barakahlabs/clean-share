package com.cleanshare.keyboard

import android.app.Activity
import android.view.View
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Plugin

/**
 * Shrinks the web view to the space above the on-screen keyboard, and back when it closes. The
 * page then never sits under the keyboard: CSS `bottom: 0` is the top of the keyboard, as on iOS
 * (KeyboardPlugin.swift does the same there).
 *
 * The app is edge-to-edge (MainActivity calls `enableEdgeToEdge()`, required from Android 15), and
 * edge-to-edge windows aren't resized for the keyboard, so `adjustResize` does nothing. The
 * activity's content is padded by the keyboard's height instead; Android animates the insets.
 */
@TauriPlugin
class KeyboardPlugin(private val activity: Activity) : Plugin(activity) {
    override fun load(webView: WebView) {
        val content = activity.findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            view.updatePadding(bottom = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom)
            insets
        }
        ViewCompat.requestApplyInsets(content)
    }
}
