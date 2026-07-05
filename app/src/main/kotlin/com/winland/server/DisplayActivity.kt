
package com.winland.server

import android.app.Activity
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import android.os.SystemClock
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import android.text.InputType
import android.util.Log
import android.util.DisplayMetrics
import android.view.HapticFeedbackConstants
import android.view.KeyEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.WindowInsets
import android.view.WindowManager
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import kotlinx.coroutines.Job
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Keyboard
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.view.WindowCompat
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.*
import com.winland.server.utils.*
import com.winland.server.engine.ChrootInstaller
import com.winland.server.ui.theme.WinlandServerTheme
import kotlinx.coroutines.launch
import java.io.File
import java.lang.ref.WeakReference
import java.util.concurrent.atomic.AtomicBoolean

class DisplayActivity : ComponentActivity() {

    private inline fun runIfNativeLoaded(actionName: String, block: () -> Unit) {
        if (!NativeBridge.isLoaded()) {
            Log.w("DisplayActivity", "Skipping $actionName: native library is not loaded yet")
            return
        }
        runCatching { block() }
            .onFailure { Log.w("DisplayActivity", "Native call failed for $actionName", it) }
    }

    private inline fun traceLifecycle(name: String, block: () -> Unit) {
        val start = SystemClock.elapsedRealtime()
        Log.i("DisplayActivity", "$name start")
        try {
            block()
        } finally {
            val elapsed = SystemClock.elapsedRealtime() - start
            Log.i("DisplayActivity", "$name end (${elapsed}ms)")
        }
    }

    // One-shot modifier: Ctrl/Alt toggled from bar sends DOWN immediately,
    // auto-releases on the next non-modifier key's UP event.
    private var ctrlOneShotPending: Boolean = false
    private var altOneShotPending: Boolean = false

    // Compose-observable state (crosses composable/Activity boundary)
    private val _ctrlActive = mutableStateOf(false)
    private val _altActive = mutableStateOf(false)

    companion object {
        private const val MOVE_DISPATCH_MIN_INTERVAL_MS = 8L
        // Reference to the active SurfaceView for drawing frames
        @Volatile
        private var activeSurfaceView: WaylandInputSurfaceView? = null
        @Volatile
        private var currentActivityRef: WeakReference<Activity>? = null

        /**
         * Called from JNI (NativeBridge.onKeyboardInitFailed) to إظهار رسالة فشل تهيئة الكيبورد.
         */
        @JvmStatic
        fun onKeyboardInitFailed(reason: String) {
            Log.e("DisplayActivity", "Keyboard initialization failed: $reason")
            val msg = if (reason.contains("keymap", true) || reason.contains("xkbcommon", true)) {
                "Keyboard initialization failed: keymap or xkbcommon files are missing in chroot. Please ensure xkb files are copied to /usr/share/X11/xkb inside chroot."
            } else {
                "Keyboard initialization failed: $reason"
            }
            // Show Toast only if we have a valid activity
            val ctx = currentActivity
            if (ctx is Activity && !ctx.isFinishing && !ctx.isDestroyed) {
                ctx.runOnUiThread {
                    try {
                        Toast.makeText(ctx, msg, Toast.LENGTH_LONG).show()
                    } catch (e: Exception) {
                        Log.w("DisplayActivity", "Safe toast failed: ${e.message}")
                    }
                }
            }
        }

        // Helper to get current activity for static calls
        val currentActivity: Activity?
            get() = try {
                currentActivityRef?.get()
            } catch (_: Exception) { null }

        /** Called from JNI when a Wayland app enables text input — show Android soft keyboard. */
        @JvmStatic
        fun showSoftKeyboard() {
            val ctx = currentActivity ?: return
            ctx.runOnUiThread {
                val imm = ctx.getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager ?: return@runOnUiThread
                val target = activeSurfaceView ?: ctx.window.decorView
                target.requestFocus()
                imm.showSoftInput(target, android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT)
            }
        }

        /** Called from JNI when a Wayland app disables text input — hide Android soft keyboard. */
        @JvmStatic
        fun hideSoftKeyboard() {
            val ctx = currentActivity ?: return
            ctx.runOnUiThread {
                val imm = ctx.getSystemService(Context.INPUT_METHOD_SERVICE) as? android.view.inputmethod.InputMethodManager ?: return@runOnUiThread
                val token = ctx.window.decorView.windowToken
                imm.hideSoftInputFromWindow(token, 0)
            }
        }
    }

    private lateinit var clipboardManager: ClipboardManager
    private val waylandClipboardListener: (String) -> Unit = { text -> updateAndroidClipboard(text) }
    @Volatile
    private var suppressNextClipboardSync = false
    @Volatile
    private var lastSyncedClipboardText: String? = null
    @Volatile
    private var clipboardListenerRegistered = false
    @Volatile
    private var isActivityForeground = false
    private val didRequestGuestStart = AtomicBoolean(false)
    private var isNativeBridgeInitialized = false
    private val primaryClipChangedListener = ClipboardManager.OnPrimaryClipChangedListener {
        if (suppressNextClipboardSync) {
            suppressNextClipboardSync = false
            return@OnPrimaryClipChangedListener
        }

        // Android restricts clipboard access for background apps.
        if (!isActivityForeground || !hasWindowFocus()) {
            // Ignore silent sync to reduce error messages
            return@OnPrimaryClipChangedListener
        }
        try {
            clipboardManager.primaryClip?.getItemAt(0)?.text?.let { text ->
                val value = text.toString()
                if (value == lastSyncedClipboardText) {
                    return@let
                }
                lastSyncedClipboardText = value
                NativeBridge.sendClipboardTextToWayland(value)
            }
        } catch (e: Exception) {
            Log.w("DisplayActivity", "Clipboard read denied", e)
        }
    }

    private var distroId: String = "ubuntu"

    override fun onCreate(savedInstanceState: Bundle?) {
        distroId = intent.getStringExtra("distro_id") ?: "ubuntu"
        Log.i("WinlandDiag", "onCreate: Entry. Distro: $distroId. Native libraries loaded: ${NativeBridge.isLoaded()}")
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        markAsCurrentActivity()

        clipboardManager = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        NativeBridge.setClipboardListener(waylandClipboardListener)

        setContent {
            var keyboardVisible by remember { mutableStateOf(false) }
            val ctrlActive by _ctrlActive
            val altActive by _altActive

            val imeCheckView = LocalView.current
            LaunchedEffect(Unit) {
                var stableSince = 0L
                while (true) {
                    delay(50)
                    val imeVisible = ViewCompat.getRootWindowInsets(imeCheckView)
                        ?.isVisible(WindowInsetsCompat.Type.ime()) ?: false
                    if (imeVisible == keyboardVisible) {
                        stableSince = 0L
                    } else if (stableSince == 0L) {
                        stableSince = SystemClock.uptimeMillis()
                    } else if (SystemClock.uptimeMillis() - stableSince > 300) {
                        keyboardVisible = imeVisible
                        stableSince = 0L
                    }
                }
            }

            // Immersive Full Screen - AndroidX Compat
            // FIXED: Safety check for decorView attachment
            SideEffect {
                WindowCompat.setDecorFitsSystemWindows(window, false)
                val controller = WindowInsetsControllerCompat(window, window.decorView)
                controller.let {
                    it.hide(WindowInsetsCompat.Type.systemBars())
                    it.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
                }
            }

            WinlandServerTheme {
                Log.i("WinlandDiag", "WinlandServerTheme: Entry")
                Box(modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black)
                ) {
                    LinuxDisplay()

                    AnimatedVisibility(
                        visible = keyboardVisible,
                        enter = slideInVertically { it },
                        exit = slideOutVertically { it },
                        modifier = Modifier
                            .align(Alignment.BottomStart)
                            .imePadding()
                    ) {
                        ExtraKeysBar(
                            ctrlActive = ctrlActive,
                            altActive = altActive,
                            onModifierToggle = { key, active ->
                                if (key == "CTRL") {
                                    _ctrlActive.value = active
                                    if (NativeBridge.isLoaded()) {
                                        NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_CTRL_LEFT, active)
                                    }
                                    ctrlOneShotPending = active
                                }
                                if (key == "ALT") {
                                    _altActive.value = active
                                    if (NativeBridge.isLoaded()) {
                                        NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_ALT_LEFT, active)
                                    }
                                    altOneShotPending = active
                                }
                            }
                        )
                    }

                    // Translucent floating keyboard button — always visible
                    SmallFloatingActionButton(
                        onClick = { toggleKeyboard() },
                        modifier = Modifier
                            .align(Alignment.BottomEnd)
                            .padding(16.dp),
                        containerColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.5f)
                    ) {
                        Icon(Icons.Default.Keyboard, contentDescription = "Toggle Keyboard")
                    }
                }
            }
        }
    }

    @Suppress("DEPRECATION")
    private fun View.hapticTap() {
        val constant = if (Build.VERSION.SDK_INT >= 30) {
            HapticFeedbackConstants.KEYBOARD_TAP
        } else {
            HapticFeedbackConstants.CONFIRM
        }
        performHapticFeedback(constant, HapticFeedbackConstants.FLAG_IGNORE_GLOBAL_SETTING)
    }

    @Composable
    fun ExtraKeysBar(
        ctrlActive: Boolean,
        altActive: Boolean,
        onModifierToggle: (key: String, active: Boolean) -> Unit,
    ) {
        val view = LocalView.current
        val regularKeys = listOf("ESC", "TAB", "↑", "↓", "←", "→")

        LazyRow(
            modifier = Modifier
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.5f))
                .padding(4.dp),
            horizontalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            item {
                Button(
                    onClick = {
                        view.hapticTap()
                        forceShowKeyboard()
                    },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.7f))
                ) {
                    Icon(Icons.Default.Keyboard, contentDescription = "Show Keyboard", tint = Color.White)
                }
            }

            // CTRL — toggle modifier with visual state
            item {
                val bgColor = if (ctrlActive) Color(0xFF00BCD4) else Color.DarkGray
                Button(
                    onClick = {
                        view.hapticTap()
                        val newState = !ctrlActive
                        onModifierToggle("CTRL", newState)
                    },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = bgColor)
                ) {
                    Text("CTRL", color = Color.White, style = MaterialTheme.typography.labelSmall,
                        fontWeight = if (ctrlActive) androidx.compose.ui.text.font.FontWeight.Bold else androidx.compose.ui.text.font.FontWeight.Normal)
                }
            }

            // ALT — toggle modifier with visual state
            item {
                val bgColor = if (altActive) Color(0xFFFF9800) else Color.DarkGray
                Button(
                    onClick = {
                        view.hapticTap()
                        val newState = !altActive
                        onModifierToggle("ALT", newState)
                    },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = bgColor)
                ) {
                    Text("ALT", color = Color.White, style = MaterialTheme.typography.labelSmall,
                        fontWeight = if (altActive) androidx.compose.ui.text.font.FontWeight.Bold else androidx.compose.ui.text.font.FontWeight.Normal)
                }
            }

            items(regularKeys) { key ->
                Button(
                    onClick = {
                        view.hapticTap()
                        simulateKey(key)
                    },
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = Color.DarkGray)
                ) {
                    Text(key, color = Color.White, style = MaterialTheme.typography.labelSmall)
                }
            }
        }
    }

    private fun simulateKey(key: String) {
        val keyCode = when(key) {
            "ESC" -> KeyEvent.KEYCODE_ESCAPE
            "TAB" -> KeyEvent.KEYCODE_TAB
            "↑" -> KeyEvent.KEYCODE_DPAD_UP
            "↓" -> KeyEvent.KEYCODE_DPAD_DOWN
            "←" -> KeyEvent.KEYCODE_DPAD_LEFT
            "→" -> KeyEvent.KEYCODE_DPAD_RIGHT
            else -> 0
        }
        if (keyCode != 0) {
            runIfNativeLoaded("simulateKey:$key") {
                NativeBridge.sendKeyEvent(keyCode, true)
                NativeBridge.sendKeyEvent(keyCode, false)
            }
        }
    }

    @Composable
    fun LinuxDisplay() {
        AndroidView(
            factory = { context ->
                Log.i("WinlandDiag", "LinuxDisplay: AndroidView Factory invoked")
                WaylandInputSurfaceView(context, ::releaseOneShotModifiers, { _ctrlActive.value }, { _altActive.value }).apply {
                    requestFocus()
                    setupLifecycle(this@DisplayActivity.lifecycleScope,
                        onSurfaceCreated = { holder ->
                            Log.i("WinlandDiag", "surfaceCreated: Surface is ready, starting native init")

                            if (!NativeBridge.isLoaded()) {
                                handleNativeInitFailure("Native libraries are not loaded")
                                return@setupLifecycle
                            }

                            // --- [Winland OS: Pre-flight Environment Validation] ---
                            val status = withContext(Dispatchers.IO) {
                                ChrootInstaller.getChrootStatus(context, distroId)
                            }
                            if (!status.ready) {
                                val err = "Environment not ready: ${status.reason}"
                                Log.e("DisplayActivity", err)
                                withContext(Dispatchers.Main) {
                                    Toast.makeText(context, err, Toast.LENGTH_LONG).show()
                                }
                                handleNativeInitFailure(err)
                                return@setupLifecycle
                            }


                            if (isNativeBridgeInitialized) {
                                Log.i("WinlandDiag", "NativeBridge already initialized, rebinding surface...")
                                NativeBridge.rebindSurface(holder.surface)
                                NativeBridge.resumeRendering()
                                return@setupLifecycle
                            }

                            val nativeInitOk = withContext(Dispatchers.IO) {
                                NativeBridge.initWaylandConnection(holder.surface, this@DisplayActivity, distroId)
                            }
                            isNativeBridgeInitialized = nativeInitOk
                            Log.i("WinlandDiag", "NativeBridge.initWaylandConnection: result=$nativeInitOk")

                            if (nativeInitOk) {
                                val refreshRate = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
                                    @Suppress("DEPRECATION")
                                    display?.mode?.refreshRate ?: 60f
                                } else {
                                    @Suppress("DEPRECATION")
                                    windowManager.defaultDisplay.refreshRate
                                }
                                NativeBridge.setRefreshRate(refreshRate)
                                android.util.Log.i("DisplayActivity", "Configured native refresh rate: $refreshRate Hz")

                                val prefs = context.getSharedPreferences("winland_settings", Context.MODE_PRIVATE)
                                NativeBridge.setScrollSensitivity(prefs.getFloat("scroll_sensitivity", 1.0f))
                                val inputPrefs = context.getSharedPreferences("winland_prefs", Context.MODE_PRIVATE)
                                NativeBridge.setInputMode(inputPrefs.getInt("input_mode_mask", 1))
                                if (didRequestGuestStart.compareAndSet(false, true)) {
                                    lifecycleScope.launch(Dispatchers.IO) {
                                        Log.i("WinlandDiag", "Guest Start: Waiting for Wayland socket probe...")

                                        var waited = 0
                                        while (!WinlandService.socketRuntimeReady && waited < 50) {
                                            delay(100)
                                            waited++
                                        }
                                        if (!WinlandService.socketRuntimeReady) {
                                            Log.w("WinlandDiag", "Guest Start: Runtime not ready after 5s, probing anyway")
                                        } else if (waited > 0) {
                                            Log.i("WinlandDiag", "Guest Start: Compositor runtime ready after ${waited * 100}ms")
                                        }

                                        val socketReady = waitForWaylandSocket(context.getUnifiedFilesDir().let { java.io.File(it) })

                                        if (socketReady) {
                                            var clientsConnected = NativeBridge.areClientsConnected()

                                            if (!clientsConnected && NativeBridge.wasSessionActive(context)) {
                                                Log.i("WinlandDiag", "Guest Start: Persistent flag shows session was active, retrying client check...")
                                                for (i in 1..5) {
                                                    delay(500)
                                                    clientsConnected = NativeBridge.areClientsConnected()
                                                    if (clientsConnected) {
                                                        Log.i("WinlandDiag", "Guest Start: Clients reconnected after ${i * 500}ms")
                                                        break
                                                    }
                                                }
                                            }

                                            if (clientsConnected) {
                                                Log.i("WinlandDiag", "Guest Start: Wayland clients already connected, desktop is running — skipping startChroot")
                                                // XWayland is already running from a previous startChroot; notify compositor.
                                                val x11Dir = context.getUnifiedFilesDir()
                                                NativeBridge.setX11SocketDir("$x11Dir/tmp")
                                                NativeBridge.notifyXwaylandReady(0)
                                                Log.i("WinlandDiag", "Guest Start: notified compositor of existing XWayland display :0")
                                            } else {
                                                Log.i("WinlandDiag", "Guest Start: Socket detected! Booting Linux desktop ($distroId)...")
                                                val res = ChrootInstaller.startChroot(context, distroId, context.resources.displayMetrics.density)
                                                if (res.isFailure) {
                                                    val err = res.exceptionOrNull()
                                                    Log.e("WinlandDiag", "Guest Start: FAILED - ${err?.message}")
                                                    withContext(Dispatchers.Main) {
                                                        Toast.makeText(context, "Linux Start Failed: ${err?.message}", Toast.LENGTH_LONG).show()
                                                    }
                                                } else {
                                                    // XWayland :0 is started explicitly inside the chroot before startxfce4.
                                                    // Give it a moment to create the socket before notifying the compositor.
                                                    val x11Dir = context.getUnifiedFilesDir()
                                                    delay(2000)
                                                    NativeBridge.setX11SocketDir("$x11Dir/tmp")
                                                    NativeBridge.notifyXwaylandReady(0)
                                                    Log.i("WinlandDiag", "Guest Start: notified compositor of XWayland display :0")
                                                }
                                            }
                                        } else {
                                            Log.e("WinlandDiag", "Guest Start: ABORTED - Wayland socket timeout")
                                            didRequestGuestStart.set(false)
                                            withContext(Dispatchers.Main) {
                                                Toast.makeText(context, "Graphics Bridge Timeout", Toast.LENGTH_LONG).show()
                                            }
                                        }
                                    }
                                }

                                withContext(Dispatchers.Main) {
                                    android.widget.Toast.makeText(context, "Wayland Engine Started", android.widget.Toast.LENGTH_SHORT).show()
                                }
                            } else {
                                handleNativeInitFailure("Failed to initialize Wayland native bridge")
                            }
                        },
                        onSurfaceDestroyed = {
                            runCatching {
                                NativeBridge.suspendRendering()
                            }.onFailure {
                                Log.w("DisplayActivity", "Immediate suspendRendering failed in surfaceDestroyed", it)
                            }
                        },
                        onSurfaceChanged = { _, format, width, height ->
                            android.util.Log.i("DisplayActivity", "com.winland.server: surfaceChanged format=$format width=$width height=$height")
                            val cutoutHeight = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                                val insets = window.decorView.rootWindowInsets
                                if (insets != null) {
                                    insets.getInsetsIgnoringVisibility(WindowInsets.Type.statusBars()).top
                                } else 0
                            } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                                window.decorView.rootWindowInsets?.displayCutout?.safeInsetTop ?: 0
                            } else 0
                            val safeWidth = width
                            val safeHeight = if (cutoutHeight > 0) height - cutoutHeight else height
                            if (cutoutHeight > 0) {
                                android.util.Log.i("DisplayActivity", "Cutout height=$cutoutHeight, safe area=${safeWidth}x${safeHeight}")
                            }
                            val metrics = DisplayMetrics()
                            @Suppress("DEPRECATION")
                            windowManager.defaultDisplay.getRealMetrics(metrics)
                            val physW = (metrics.widthPixels * 25.4f / metrics.xdpi).toInt()
                            val physH = (metrics.heightPixels * 25.4f / metrics.ydpi).toInt()
                            runIfNativeLoaded("onSurfaceChanged") {
                                NativeBridge.onSurfaceChanged(safeWidth, safeHeight, physW, physH)
                                if (cutoutHeight > 0) {
                                    NativeBridge.setYOffset(cutoutHeight)
                                }
                            }
                        }
                    )
                }
                },
            modifier = Modifier.fillMaxSize()
        )
    }

    private fun handleNativeInitFailure(message: String) {
        didRequestGuestStart.set(false)
        val diag = "com.winland.server: native init failure: $message"
        android.util.Log.e("DisplayActivity", diag)
        appendWaylandDiag(diag)
        // Do not try to show Toast if Activity is finishing
        if (!isFinishing && !isDestroyed) {
            runOnUiThread {
                Toast.makeText(this, message, Toast.LENGTH_LONG).show()
                finish()
            }
        }
    }

    private fun appendWaylandDiag(message: String) {
        runCatching {
            val tmpDir = File(filesDir, "tmp")
            if (!tmpDir.exists()) {
                tmpDir.mkdirs()
            }
            val logFile = File(tmpDir, "wayland-debug.txt")
            logFile.appendText("${System.currentTimeMillis()} $message\n")
        }
    }

    private fun setupClipboardListener() {
        if (!clipboardListenerRegistered) {
            clipboardManager.addPrimaryClipChangedListener(primaryClipChangedListener)
            clipboardListenerRegistered = true
        }
    }

    private fun teardownClipboardListener() {
        if (clipboardListenerRegistered) {
            clipboardManager.removePrimaryClipChangedListener(primaryClipChangedListener)
            clipboardListenerRegistered = false
        }
    }

    // This could be called from NativeBridge.onWaylandClipboardChanged
    fun updateAndroidClipboard(text: String) {
        runOnUiThread {
            if (text == lastSyncedClipboardText) {
                return@runOnUiThread
            }

            // Android security: Cannot set clipboard unless in foreground and focused.
            if (!isActivityForeground || !hasWindowFocus()) {
                Log.d("DisplayActivity", "Ignoring background clipboard update from Wayland")
                return@runOnUiThread
            }

            try {
                suppressNextClipboardSync = true
                lastSyncedClipboardText = text
                val clip = ClipData.newPlainText("Wayland", text)
                clipboardManager.setPrimaryClip(clip)
                Log.i("DisplayActivity", "Synced Wayland clipboard to Android")
            } catch (e: Exception) {
                Log.w("DisplayActivity", "Clipboard access denied (likely focus lost)", e)
                suppressNextClipboardSync = false
            }
        }
    }

    override fun onDestroy() {
        traceLifecycle("onDestroy") {
            clearCurrentActivityIfSelf()
            NativeBridge.setClipboardListener(null)
            teardownClipboardListener()
            super.onDestroy()
        }
    }

    override fun onStart() {
        traceLifecycle("onStart") {
            super.onStart()
            markAsCurrentActivity()
        }
    }

    override fun onResume() {
        traceLifecycle("onResume") {
            super.onResume()
            isActivityForeground = true
            markAsCurrentActivity()
            setupClipboardListener()
            // resumeRendering() is idempotent — it only flips RENDERING_ACTIVE to true.
            // The compositor also checks egl_surface validity before actually drawing,
            // so this is safe to call even if the EGL surface isn't ready yet.
            // Needed here because surfaceCreated() does NOT always fire on resume
            // (Android may keep the Surface alive during background). Without this,
            // RENDERING_ACTIVE stays false from onStop() and the screen stays black.
            runCatching {
                NativeBridge.resumeRendering()
            }.onFailure {
                Log.w("DisplayActivity", "resumeRendering in onResume failed", it)
            }
        }
    }

    override fun onStop() {
        traceLifecycle("onStop") {
            isActivityForeground = false
            teardownClipboardListener()
            setNativeRenderingActiveAsync(active = false)
            clearCurrentActivityIfSelf()
            super.onStop()
        }
    }

    private fun setNativeRenderingActiveAsync(active: Boolean) {
        lifecycleScope.launch(Dispatchers.Default) {
            runCatching {
                if (active) {
                    NativeBridge.resumeRendering()
                } else {
                    NativeBridge.suspendRendering()
                }
            }.onFailure {
                Log.w("DisplayActivity", "Failed to toggle rendering state active=$active", it)
            }
        }
    }

    private fun markAsCurrentActivity() {
        currentActivityRef = WeakReference(this)
        Log.d("DisplayActivity", "Current activity attached")
    }

    private fun clearCurrentActivityIfSelf() {
        val current = currentActivityRef?.get()
        if (current === this) {
            currentActivityRef = null
            Log.d("DisplayActivity", "Current activity detached")
        }
    }

    private fun toggleKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        val target = activeSurfaceView ?: window.decorView
        val decor = window.decorView
        val imeVisible = ViewCompat.getRootWindowInsets(decor)?.isVisible(WindowInsetsCompat.Type.ime()) == true
        if (imeVisible) {
            imm.hideSoftInputFromWindow(decor.windowToken, 0)
        } else {
            target.requestFocus()
            imm.showSoftInput(target, InputMethodManager.SHOW_IMPLICIT)
        }
    }

    // SHOW_FORCED is deprecated in API 33 but intentionally used here: the force-keyboard button
    // is specifically for use in games/apps that don't declare text fields, so forcing is correct.
    @Suppress("DEPRECATION")
    private fun forceShowKeyboard() {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        val target = activeSurfaceView ?: window.decorView
        target.requestFocus()
        imm.showSoftInput(target, InputMethodManager.SHOW_FORCED)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        // Surface resize is handled by SurfaceHolder.Callback.onSurfaceChanged().
        // Immersive mode (WindowInsetsControllerCompat) is re-applied by the
        // SideEffect block in the Compose layout on recomposition.
    }

    private fun isModifierKey(keyCode: Int): Boolean = keyCode in listOf(
        KeyEvent.KEYCODE_CTRL_LEFT, KeyEvent.KEYCODE_CTRL_RIGHT,
        KeyEvent.KEYCODE_ALT_LEFT, KeyEvent.KEYCODE_ALT_RIGHT,
        KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent.KEYCODE_SHIFT_RIGHT,
        KeyEvent.KEYCODE_META_LEFT, KeyEvent.KEYCODE_META_RIGHT,
        KeyEvent.KEYCODE_SYM, KeyEvent.KEYCODE_FUNCTION
    )

    override fun onKeyDown(keyCode: Int, event: KeyEvent?): Boolean {
        runIfNativeLoaded("onKeyDown:$keyCode") {
            NativeBridge.sendKeyEvent(keyCode, true)
        }
        return super.onKeyDown(keyCode, event)
    }

    override fun onKeyUp(keyCode: Int, event: KeyEvent?): Boolean {
        runIfNativeLoaded("onKeyUp:$keyCode") {
            NativeBridge.sendKeyEvent(keyCode, false)
        }

        if (ctrlOneShotPending && !isModifierKey(keyCode)) {
            ctrlOneShotPending = false
            _ctrlActive.value = false
            if (NativeBridge.isLoaded()) {
                NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_CTRL_LEFT, false)
            }
        }
        if (altOneShotPending && !isModifierKey(keyCode)) {
            altOneShotPending = false
            _altActive.value = false
            if (NativeBridge.isLoaded()) {
                NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_ALT_LEFT, false)
            }
        }

        return super.onKeyUp(keyCode, event)
    }

    private fun releaseOneShotModifiers() {
        if (ctrlOneShotPending) {
            ctrlOneShotPending = false
            runOnUiThread { _ctrlActive.value = false }
            if (NativeBridge.isLoaded()) {
                NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_CTRL_LEFT, false)
            }
        }
        if (altOneShotPending) {
            altOneShotPending = false
            runOnUiThread { _altActive.value = false }
            if (NativeBridge.isLoaded()) {
                NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_ALT_LEFT, false)
            }
        }
    }

    @android.annotation.SuppressLint("ClickableViewAccessibility")
    private class WaylandInputSurfaceView(context: Context, private val onImeCommit: () -> Unit, private val ctrlActive: () -> Boolean, private val altActive: () -> Boolean) : SurfaceView(context) {
        companion object {
            private fun charToKeyCode(ch: Char): Int? = when (ch.uppercaseChar()) {
                'A' -> KeyEvent.KEYCODE_A; 'B' -> KeyEvent.KEYCODE_B
                'C' -> KeyEvent.KEYCODE_C; 'D' -> KeyEvent.KEYCODE_D
                'E' -> KeyEvent.KEYCODE_E; 'F' -> KeyEvent.KEYCODE_F
                'G' -> KeyEvent.KEYCODE_G; 'H' -> KeyEvent.KEYCODE_H
                'I' -> KeyEvent.KEYCODE_I; 'J' -> KeyEvent.KEYCODE_J
                'K' -> KeyEvent.KEYCODE_K; 'L' -> KeyEvent.KEYCODE_L
                'M' -> KeyEvent.KEYCODE_M; 'N' -> KeyEvent.KEYCODE_N
                'O' -> KeyEvent.KEYCODE_O; 'P' -> KeyEvent.KEYCODE_P
                'Q' -> KeyEvent.KEYCODE_Q; 'R' -> KeyEvent.KEYCODE_R
                'S' -> KeyEvent.KEYCODE_S; 'T' -> KeyEvent.KEYCODE_T
                'U' -> KeyEvent.KEYCODE_U; 'V' -> KeyEvent.KEYCODE_V
                'W' -> KeyEvent.KEYCODE_W; 'X' -> KeyEvent.KEYCODE_X
                'Y' -> KeyEvent.KEYCODE_Y; 'Z' -> KeyEvent.KEYCODE_Z
                else -> null
            }
        }
        private var lastMoveDispatchUptimeMs: Long = 0L
        private var surfaceJob: Job? = null

        private val mainHandler = android.os.Handler(context.mainLooper)

        private val LONG_PRESS_MS = 400L
        private val LONG_PRESS_MOVE_THRESHOLD_PX = 20f

        private enum class GestureState {
            IDLE,
            TAP_PENDING,
            DRAG_ACTIVE,
            MOVING
        }

        private var gestureState = GestureState.IDLE
        private var primaryPointerId = -1
        private var primaryDownX = 0f
        private var primaryDownY = 0f
        private var lastDragX = 0f
        private var lastDragY = 0f

        private val longPressRunnable = Runnable {
            if (gestureState != GestureState.TAP_PENDING) return@Runnable
            gestureState = GestureState.DRAG_ACTIVE
        }

        init {
            holder.setFormat(android.graphics.PixelFormat.RGBA_8888)
            isFocusable = true
            isFocusableInTouchMode = true
            activeSurfaceView = this
        }

        fun setupLifecycle(
            lifecycleScope: androidx.lifecycle.LifecycleCoroutineScope,
            onSurfaceCreated: suspend (holder: SurfaceHolder) -> Unit,
            onSurfaceChanged: (holder: SurfaceHolder, format: Int, width: Int, height: Int) -> Unit,
            onSurfaceDestroyed: () -> Unit
        ) {
            holder.addCallback(object : SurfaceHolder.Callback {
                override fun surfaceCreated(holder: SurfaceHolder) {
                    Log.i("DisplayActivity", "com.winland.server: surfaceCreated (setupLifecycle)")
                    activeSurfaceView = this@WaylandInputSurfaceView
                    surfaceJob?.cancel()
                    holder.setFormat(android.graphics.PixelFormat.RGBA_8888)
                    surfaceJob = lifecycleScope.launch(Dispatchers.Main) {
                        delay(300)
                        if (!holder.surface.isValid) {
                            Log.w("DisplayActivity", "surfaceCreated coroutine: Surface is no longer valid, skipping")
                            return@launch
                        }
                        onSurfaceCreated(holder)
                    }
                }
                override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
                    onSurfaceChanged(holder, format, width, height)
                }
                override fun surfaceDestroyed(holder: SurfaceHolder) {
                    Log.w("DisplayActivity", "com.winland.server: surfaceDestroyed (setupLifecycle)")
                    surfaceJob?.cancel()
                    surfaceJob = null
                    onSurfaceDestroyed()
                    activeSurfaceView = null
                }
            })
        }

        @android.annotation.SuppressLint("ClickableViewAccessibility")
        override fun onTouchEvent(event: android.view.MotionEvent): Boolean {
            if (!holder.surface.isValid) {
                return true
            }

            val actionMasked = event.actionMasked

            // Two-finger right-click: intercept before trackpad routing to avoid
            // lifecycle issues with the second finger's touch ID.
            if (actionMasked == android.view.MotionEvent.ACTION_POINTER_DOWN && event.pointerCount >= 2) {
                if (NativeBridge.isLoaded()) {
                    val now = (SystemClock.uptimeMillis() and 0x7FFFFFFF).toInt()
                    NativeBridge.sendTrackpadClick(1, 0x111, now)
                    this.postDelayed({
                        NativeBridge.sendTrackpadClick(0, 0x111,
                            (SystemClock.uptimeMillis() and 0x7FFFFFFF).toInt())
                    }, 50)
                }
                return true
            }

            when (actionMasked) {
                android.view.MotionEvent.ACTION_DOWN -> {
                    parent?.requestDisallowInterceptTouchEvent(true)
                    requestFocus()
                    val i = event.actionIndex
                    val pointerId = event.getPointerId(i)
                    val x = event.getX(i)
                    val y = event.getY(i)

                    gestureState = GestureState.TAP_PENDING
                    primaryPointerId = pointerId
                    primaryDownX = x
                    primaryDownY = y
                    lastDragX = x
                    lastDragY = y

                    if (NativeBridge.isLoaded()) {
                        NativeBridge.sendTouchEvent(actionMasked, pointerId, x, y)
                    }
                    mainHandler.postDelayed(longPressRunnable, LONG_PRESS_MS)
                }

                android.view.MotionEvent.ACTION_POINTER_DOWN -> {
                    if (gestureState == GestureState.TAP_PENDING || gestureState == GestureState.DRAG_ACTIVE) {
                        mainHandler.removeCallbacks(longPressRunnable)
                        if (NativeBridge.isLoaded()) {
                            val now = (SystemClock.uptimeMillis() and 0x7FFFFFFF).toInt()
                            NativeBridge.sendTrackpadClick(1, 3, now)
                            NativeBridge.sendTrackpadClick(0, 3, now)
                            NativeBridge.sendTouchEvent(
                                android.view.MotionEvent.ACTION_CANCEL,
                                primaryPointerId, primaryDownX, primaryDownY
                            )
                        }
                        gestureState = GestureState.IDLE
                    }
                }

                android.view.MotionEvent.ACTION_MOVE -> {
                    if (gestureState == GestureState.TAP_PENDING) {
                        var movedEnough = false
                        for (i in 0 until event.pointerCount) {
                            val pid = event.getPointerId(i)
                            if (pid == primaryPointerId) {
                                val dx = event.getX(i) - primaryDownX
                                val dy = event.getY(i) - primaryDownY
                                if (dx * dx + dy * dy > LONG_PRESS_MOVE_THRESHOLD_PX * LONG_PRESS_MOVE_THRESHOLD_PX) {
                                    movedEnough = true
                                }
                            }
                        }
                        if (movedEnough) {
                            mainHandler.removeCallbacks(longPressRunnable)
                            if (NativeBridge.isLoaded()) {
                                NativeBridge.sendTouchEvent(
                                    android.view.MotionEvent.ACTION_CANCEL,
                                    primaryPointerId, primaryDownX, primaryDownY
                                )
                            }
                            for (i in 0 until event.pointerCount) {
                                val pid = event.getPointerId(i)
                                if (pid == primaryPointerId) {
                                    lastDragX = event.getX(i)
                                    lastDragY = event.getY(i)
                                }
                            }
                            gestureState = GestureState.MOVING
                        }
                    }

                    if (gestureState == GestureState.DRAG_ACTIVE) {
                        for (i in 0 until event.pointerCount) {
                            val pid = event.getPointerId(i)
                            if (pid == primaryPointerId) {
                                if (NativeBridge.isLoaded()) {
                                    NativeBridge.sendTouchEvent(
                                        android.view.MotionEvent.ACTION_MOVE,
                                        pid, event.getX(i), event.getY(i)
                                    )
                                }
                            }
                        }
                    } else if (gestureState == GestureState.MOVING) {
                        for (i in 0 until event.pointerCount) {
                            val pid = event.getPointerId(i)
                            if (pid == primaryPointerId) {
                                val dx = event.getX(i) - lastDragX
                                val dy = event.getY(i) - lastDragY
                                lastDragX = event.getX(i)
                                lastDragY = event.getY(i)
                                if (NativeBridge.isLoaded()) {
                                    val now = (SystemClock.uptimeMillis() and 0x7FFFFFFF).toInt()
                                    NativeBridge.sendRelativeMotion(dx, dy, now)
                                }
                            }
                        }
                    }
                }

                android.view.MotionEvent.ACTION_UP -> {
                    mainHandler.removeCallbacks(longPressRunnable)
                    val i = event.actionIndex
                    val pointerId = event.getPointerId(i)
                    val x = event.getX(i)
                    val y = event.getY(i)
                    if (gestureState == GestureState.TAP_PENDING) {
                        if (NativeBridge.isLoaded()) {
                            NativeBridge.sendTouchEvent(actionMasked, pointerId, x, y)
                        }
                    } else if (gestureState == GestureState.DRAG_ACTIVE && pointerId == primaryPointerId) {
                        if (NativeBridge.isLoaded()) {
                            NativeBridge.sendTouchEvent(actionMasked, pointerId, x, y)
                        }
                    }
                    gestureState = GestureState.IDLE
                    parent?.requestDisallowInterceptTouchEvent(false)
                    performClick()
                }

                android.view.MotionEvent.ACTION_CANCEL -> {
                    mainHandler.removeCallbacks(longPressRunnable)
                    gestureState = GestureState.IDLE
                    parent?.requestDisallowInterceptTouchEvent(false)
                    for (i in 0 until event.pointerCount) {
                        val pointerId = event.getPointerId(i)
                        val x = event.getX(i)
                        val y = event.getY(i)
                        if (NativeBridge.isLoaded()) {
                            NativeBridge.sendTouchEvent(android.view.MotionEvent.ACTION_CANCEL,
                                pointerId, x, y)
                        }
                    }
                }
            }
            return true
        }

        override fun onDetachedFromWindow() {
            if (activeSurfaceView === this) activeSurfaceView = null
            super.onDetachedFromWindow()
        }

        override fun onCheckIsTextEditor(): Boolean = true

        override fun performClick(): Boolean {
            super.performClick()
            return true
        }

        override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
            outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
            outAttrs.imeOptions = EditorInfo.IME_ACTION_NONE

            return object : BaseInputConnection(this, false) {
                override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
                    if (!text.isNullOrEmpty() && NativeBridge.isLoaded()) {
                        val str = text.toString()
                        val ctrl = ctrlActive()
                        val alt = altActive()

                        if ((ctrl || alt) && str.length == 1 && str[0].isLetter()) {
                            val keyCode = charToKeyCode(str[0])
                            if (keyCode != null) {
                                if (ctrl) NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_CTRL_LEFT, true)
                                if (alt) NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_ALT_LEFT, true)
                                NativeBridge.sendKeyEvent(keyCode, true)
                                NativeBridge.sendKeyEvent(keyCode, false)
                                if (alt) NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_ALT_LEFT, false)
                                if (ctrl) NativeBridge.sendKeyEvent(KeyEvent.KEYCODE_CTRL_LEFT, false)
                                onImeCommit()
                                return true
                            }
                        }

                        NativeBridge.sendTextInput(str)
                    }
                    onImeCommit()
                    return true
                }

                override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
                    if (!text.isNullOrEmpty() && NativeBridge.isLoaded()) {
                        NativeBridge.sendTextInput(text.toString())
                    }
                    return true
                }

                override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
                    if (beforeLength > 0) {
                        if (NativeBridge.isLoaded()) {
                            NativeBridge.sendTextInput("\b")
                        }
                    }
                    return true
                }

                override fun sendKeyEvent(event: KeyEvent): Boolean {
                    if (NativeBridge.isLoaded()) {
                        NativeBridge.sendKeyEvent(event.keyCode, event.action == KeyEvent.ACTION_DOWN)
                    }
                    if (event.action == KeyEvent.ACTION_UP) {
                        onImeCommit()
                    }
                    return true
                }
            }
        }
    }
    private suspend fun waitForWaylandSocket(filesDir: File): Boolean {
        val socketPath = File(filesDir, "tmp/wayland-0")
        Log.i("WinlandDiag", "waitForWaylandSocket: Probing path=${socketPath.absolutePath}")
        
        repeat(150) { i ->
            val exists = socketPath.exists()
            val isSock = isUnixSocket(socketPath)
            
            if (exists || isSock) {
                Log.i("WinlandDiag", "waitForWaylandSocket: SUCCESS at iteration $i! exists=$exists, isSock=$isSock")
                return true
            }
            if (i % 20 == 0) {
                Log.d("WinlandDiag", "waitForWaylandSocket: Still waiting... iter=$i")
            }
            delay(100)
        }
        
        val finalExists = socketPath.exists()
        val finalIsSock = isUnixSocket(socketPath)
        Log.e("WinlandDiag", "waitForWaylandSocket: TIMEOUT! exists=$finalExists, isSock=$finalIsSock")
        return finalExists || finalIsSock
    }

    private fun isUnixSocket(file: File): Boolean {
        return try {
            val stat = Os.lstat(file.absolutePath)
            OsConstants.S_ISSOCK(stat.st_mode)
        } catch (_: ErrnoException) {
            false
        } catch (_: Exception) {
            false
        }
    }
}
