package com.cleanshare.appearance

import android.app.Activity
import android.content.res.Configuration
import androidx.core.view.WindowCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class SetModeArgs {
    var mode: String = "system"
}

/**
 * Sets the status and navigation bar icons to match the mode chosen in the app: dark icons on a
 * light app, light icons on a dark one. The app is edge-to-edge, so the bars are see-through and
 * only their icons have a colour. "system" follows the phone's setting.
 */
@TauriPlugin
class AppearancePlugin(private val activity: Activity) : Plugin(activity) {
    @Command
    fun setMode(invoke: Invoke) {
        val mode = invoke.parseArgs(SetModeArgs::class.java).mode
        val dark = when (mode) {
            "light" -> false
            "dark" -> true
            else -> (activity.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
                Configuration.UI_MODE_NIGHT_YES
        }
        activity.runOnUiThread {
            WindowCompat.getInsetsController(activity.window, activity.window.decorView).apply {
                isAppearanceLightStatusBars = !dark
                isAppearanceLightNavigationBars = !dark
            }
            invoke.resolve()
        }
    }
}
