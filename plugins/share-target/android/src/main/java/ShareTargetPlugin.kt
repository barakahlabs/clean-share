package com.cleanshare.sharetarget

import android.app.Activity
import android.content.Intent
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

/**
 * Receives text shared via the Android share sheet (ACTION_SEND, text/plain).
 *
 * The shared text is held until JS reads it with `get_shared`. When a share
 * arrives while the app is running, a `shared` event tells JS to read it.
 */
@TauriPlugin
class ShareTargetPlugin(private val activity: Activity) : Plugin(activity) {
    private var pendingText: String? = null

    // Cold start: the app was launched by the share sheet.
    override fun load(webView: WebView) {
        takeSharedText(activity.intent)
    }

    // Warm start: MainActivity is singleTask, so new shares arrive here.
    override fun onNewIntent(intent: Intent) {
        if (takeSharedText(intent)) {
            trigger("shared", JSObject())
        }
    }

    @Command
    fun getShared(invoke: Invoke) {
        val result = JSObject()
        result.put("text", pendingText)
        pendingText = null
        invoke.resolve(result)
    }

    private fun takeSharedText(intent: Intent?): Boolean {
        if (intent?.action != Intent.ACTION_SEND) return false
        if (intent.type?.startsWith("text/") != true) return false
        val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()
        if (text.isNullOrEmpty()) return false

        pendingText = text
        // Mark as handled so the same share isn't delivered again.
        intent.action = null
        return true
    }
}
