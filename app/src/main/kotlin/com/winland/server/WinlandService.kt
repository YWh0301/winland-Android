package com.winland.server

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.os.SystemClock
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.lifecycle.LifecycleService
import androidx.lifecycle.lifecycleScope
import com.winland.server.engine.WinlandAudioServer
import com.winland.server.engine.WinlandCameraBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

class WinlandService : LifecycleService() {

    private val TAG = "WinlandService"
    private val audioServer = WinlandAudioServer()
    private lateinit var cameraBridge: WinlandCameraBridge
    private var wakeLock: PowerManager.WakeLock? = null

    @Volatile
    private var runtimeStarted = false

    companion object {
        @Volatile
        var socketRuntimeReady = false
            private set
    }

    private inline fun <T> traceLifecycle(name: String, block: () -> T): T {
        val start = SystemClock.elapsedRealtime()
        Log.i(TAG, "$name start")
        return try {
            block()
        } finally {
            val elapsed = SystemClock.elapsedRealtime() - start
            Log.i(TAG, "$name end (${elapsed}ms)")
        }
    }

    override fun onCreate() {
        traceLifecycle("onCreate") {
            super.onCreate()
            Log.i(TAG, "WinlandService Created")
            val activeDistroId = getSharedPreferences("winland_prefs", MODE_PRIVATE)
                .getString("active_distro_id", "ubuntu") ?: "ubuntu"
            val ok = try {
                NativeBridge.ensureSocketRuntime(this, activeDistroId)
            } catch (t: Throwable) {
                Log.e(TAG, "ensureSocketRuntime onCreate failed", t)
                false
            }
            socketRuntimeReady = ok
            Log.i(TAG, "ensureSocketRuntime onCreate result=$ok (distro=$activeDistroId)")
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return traceLifecycle("onStartCommand") {
            super.onStartCommand(intent, flags, startId)

            // Must be called quickly after startForegroundService() to avoid system kill.
            val notification = createNotification()
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
                startForeground(
                    1,
                    notification,
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
                )
            } else {
                startForeground(1, notification)
            }

            // Acquire partial wake lock to keep the process alive across Activity restarts.
            acquireWakeLock()

            // Update compositor XKB config for current distro (compositor may already be running).
            val activeDistroId = getSharedPreferences("winland_prefs", MODE_PRIVATE)
                .getString("active_distro_id", "ubuntu") ?: "ubuntu"
            val ok = try {
                NativeBridge.ensureSocketRuntime(this, activeDistroId)
            } catch (t: Throwable) {
                Log.e(TAG, "ensureSocketRuntime onStartCommand failed", t)
                false
            }
            socketRuntimeReady = ok
            Log.i(TAG, "ensureSocketRuntime onStartCommand result=$ok (distro=$activeDistroId)")

            if (!runtimeStarted) {
                runtimeStarted = true
                lifecycleScope.launch(Dispatchers.Default) {
                    try {
                        if (!NativeBridge.awaitLibrariesLoaded()) {
                            Log.e(TAG, "Native libraries not loaded; service running in degraded mode")
                        }
                        audioServer.start()
//                        cameraBridge = WinlandCameraBridge(this@WinlandService)
//                        cameraBridge.start(this@WinlandService)
                    } catch (t: Throwable) {
                        Log.e(TAG, "Failed starting runtime components", t)
                    }
                }
            }

            START_STICKY
        }
    }

    private fun createNotification(): Notification {
        val channelId = "winland_service"
        val channel = NotificationChannel(channelId, "Winland Server", NotificationManager.IMPORTANCE_LOW)
        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        manager.createNotificationChannel(channel)

        return NotificationCompat.Builder(this, channelId)
            .setContentTitle("Winland Server - Active")
            .setContentText("Linux session running. Stop service before uninstalling to protect /sdcard.")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setOngoing(true)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .build()
    }

    override fun onBind(intent: Intent): IBinder? {
        super.onBind(intent)
        return null
    }

    private fun acquireWakeLock() {
        try {
            val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
            wakeLock = powerManager.newWakeLock(
                PowerManager.PARTIAL_WAKE_LOCK,
                "WinlandService:CompositorLock"
            )
            wakeLock?.acquire()
            Log.i(TAG, "Wake lock acquired")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to acquire wake lock", e)
        }
    }

    override fun onDestroy() {
        traceLifecycle("onDestroy") {
            // Release wake lock
            try {
                wakeLock?.release()
                wakeLock = null
                Log.i(TAG, "Wake lock released")
            } catch (_: Exception) {}

            // Full teardown: stop compositor thread, unbind socket, release native resources.
            // This is a true service shutdown — not a freeze.
            NativeBridge.releaseWaylandConnection()

            runCatching { audioServer.stop() }
            if (::cameraBridge.isInitialized) {
                runCatching { cameraBridge.stop() }
            }

            // Safe unmount of all chroot bind-mounts (proc/sys/dev/sdcard/dev-shm)
            // This protects /sdcard data if the app is force-stopped or uninstalled,
            // without blocking service teardown on the main thread.
            CleanupReceiver.safeUnmountAsync(this)
            Log.i(TAG, "WinlandService Destroyed - rendering suspended, audio/camera stopped, mounts cleanup scheduled")
            super.onDestroy()
        }
    }
}
