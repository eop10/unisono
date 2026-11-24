package app.unisono.tuner

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.view.WindowManager
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin
import app.tauri.plugin.PluginManager

@TauriPlugin
class MicPlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun requestMic(invoke: Invoke) {
        if (ContextCompat.checkSelfPermission(activity, Manifest.permission.RECORD_AUDIO)
            == PackageManager.PERMISSION_GRANTED
        ) {
            invoke.resolve()
            return
        }

        activity.runOnUiThread {
            PluginManager.requestPermissions(arrayOf(Manifest.permission.RECORD_AUDIO)) { results ->
                val granted = results[Manifest.permission.RECORD_AUDIO] == true
                if (granted) {
                    invoke.resolve()
                } else {
                    invoke.reject("permission denied")
                }
            }
        }
    }

    @Command
    fun keepScreenOn(invoke: Invoke) {
        val enabled = try {
            invoke.getArgs().optBoolean("enabled", true)
        } catch (_: Exception) {
            true
        }
        activity.runOnUiThread {
            if (enabled) {
                activity.window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            } else {
                activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            }
        }
        invoke.resolve()
    }

    @Command
    fun vibrate(invoke: Invoke) {
        val v = activity.getSystemService(Vibrator::class.java)
        if (v != null) {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                v.vibrate(VibrationEffect.createOneShot(20, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                v.vibrate(20)
            }
        }
        invoke.resolve()
    }

    override fun onPause(activity: AppCompatActivity) {
        super.onPause(activity)
        activity.runOnUiThread {
            activity.window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        }
    }
}
