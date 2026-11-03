package com.cleanshare.sharetarget

import android.app.Activity
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import androidx.core.content.FileProvider
import java.io.File
import android.os.Build
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ShareFileArgs {
    lateinit var path: String
    lateinit var mimeType: String
}

@InvokeArg
class ShareTextArgs {
    lateinit var text: String
}

/**
 * Receives text shared via the Android share sheet (ACTION_SEND, text/plain).
 *
 * The shared text is held until JS reads it with `get_shared`. When a share
 * arrives while the app is running, a `shared` event tells JS to read it.
 *
 * Each share-sheet entry is its own component: MainActivity is "Clean & Share",
 * and an `activity-alias` whose name ends in [SAVE_ALIAS] is "Save bookmark".
 * The component the share arrived on tells us which one the user picked.
 */
@TauriPlugin
class ShareTargetPlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        /** Class-name suffix of the "Save bookmark" activity-alias in AndroidManifest.xml. */
        const val SAVE_ALIAS = ".SaveBookmark"
    }

    private var pendingText: String? = null
    private var pendingIntent: String = "share"

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

    // Opens the system share sheet so the user can send text to another app.
    @Command
    fun shareText(invoke: Invoke) {
        val args = invoke.parseArgs(ShareTextArgs::class.java)
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, args.text)
        }
        val chooser = Intent.createChooser(send, null).apply {
            // Don't offer sharing back into this app.
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
                putExtra(
                    Intent.EXTRA_EXCLUDE_COMPONENTS,
                    arrayOf(ComponentName(activity, activity.javaClass))
                )
            }
        }
        activity.runOnUiThread {
            activity.startActivity(chooser)
            invoke.resolve()
        }
    }

    // Goes back to the app that shared: the task moves to the background, which shows the app
    // underneath it (the source app, since the share started this task or brought it forward).
    @Command
    fun returnToSource(invoke: Invoke) {
        activity.runOnUiThread {
            val result = JSObject()
            result.put("returned", activity.moveTaskToBack(true))
            invoke.resolve(result)
        }
    }

    /**
     * Opens the share sheet with a file from the app's cache dir. Uses the FileProvider that
     * Tauri's Android template declares (`<package>.fileprovider`, whose paths include the cache
     * dir); a second provider here would clash with it in the manifest merge.
     */
    @Command
    fun shareFile(invoke: Invoke) {
        val args = invoke.parseArgs(ShareFileArgs::class.java)
        val uri = try {
            FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", File(args.path))
        } catch (e: IllegalArgumentException) {
            invoke.reject("Can't share ${args.path}: ${e.message}")
            return
        }
        val send = Intent(Intent.ACTION_SEND).apply {
            type = args.mimeType
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        activity.runOnUiThread {
            activity.startActivity(Intent.createChooser(send, null))
            invoke.resolve()
        }
    }

    /** Whether the current connection is metered (mobile data, a hotspot) or there's none. */
    @Command
    fun isMetered(invoke: Invoke) {
        val connectivity = activity.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val result = JSObject()
        result.put("metered", connectivity.activeNetwork == null || connectivity.isActiveNetworkMetered)
        invoke.resolve(result)
    }

    @Command
    fun getShared(invoke: Invoke) {
        val result = JSObject()
        pendingText?.let {
            result.put("text", it)
            result.put("intent", pendingIntent)
        }
        pendingText = null
        invoke.resolve(result)
    }

    private fun takeSharedText(intent: Intent?): Boolean {
        if (intent?.action != Intent.ACTION_SEND) return false
        if (intent.type?.startsWith("text/") != true) return false
        val text = intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()
        if (text.isNullOrEmpty()) return false

        pendingText = text
        val component = intent.component?.className.orEmpty()
        pendingIntent = if (component.endsWith(SAVE_ALIAS)) "save" else "share"
        // Mark as handled so the same share isn't delivered again.
        intent.action = null
        return true
    }
}
