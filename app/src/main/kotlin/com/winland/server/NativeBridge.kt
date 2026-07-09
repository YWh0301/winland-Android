package com.winland.server

import android.util.Log
import android.view.Surface
import dalvik.system.BaseDexClassLoader
import com.winland.server.utils.getUnifiedFilesDir

object NativeBridge {
    private const val TAG = "NativeBridge"

    /**
     * Called from native (JNI) when keyboard initialization fails (libxkbcommon/keymap).
     */
    @JvmStatic
    fun onKeyboardInitFailed(reason: String) {
        Log.e(TAG, "Keyboard initialization failed: $reason")
        DisplayActivity.onKeyboardInitFailed(reason)
    }

    /**
     * Called from native (JNI) to deliver a new frame (ARGB8888) to the UI.
     * @param pixels ByteArray containing ARGB8888 pixels
     * @param width Frame width
     * @param height Frame height
     */
    @JvmStatic
    external fun onFrameReceived(pixels: ByteArray, width: Int, height: Int)

    private var isLibLoaded = false
    private var isLibLoading = false
    private val loadingLock = Any()

    @Volatile
    private var clipboardListener: ((String) -> Unit)? = null

    private fun loadLibraryStrict(name: String) {
        val loader = NativeBridge::class.java.classLoader
        try {
            Log.i(TAG, "Loading $name via System.loadLibrary")
            System.loadLibrary(name)
        } catch (e: UnsatisfiedLinkError) {
            val dexLoader = loader as? BaseDexClassLoader
            val absolutePath = dexLoader?.findLibrary(name)
            if (!absolutePath.isNullOrBlank()) {
                Log.i(TAG, "Loading $name from absolute path: $absolutePath")
                System.load(absolutePath)
            } else {
                throw e
            }
        }
    }

    /**
     * Preload native libraries asynchronously. Call this early in app startup (e.g., from Application.onCreate).
     * This avoids blocking the main thread on xkbcommon initialization (30+ seconds on first load).
     */
    fun preloadLibraries() {
        synchronized(loadingLock) {
            if (isLibLoaded || isLibLoading) {
                Log.i(TAG, "preloadLibraries: already loaded or loading, skipping")
                return
            }
            isLibLoading = true
        }

        try {
            Log.i(TAG, "Starting async native library preload: c++_shared → xkbcommon → uniffi_winland_core")
            val startTime = System.currentTimeMillis()
            
            // Keep explicit order: core C++ runtime -> xkbcommon provider -> rust cdylib consumer.
            loadLibraryStrict("c++_shared")
            loadLibraryStrict("xkbcommon")
            loadLibraryStrict("uniffi_winland_core")
            
            val elapsedMs = System.currentTimeMillis() - startTime
            isLibLoaded = true
            Log.i(TAG, "Native libraries loaded successfully in ${elapsedMs}ms")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load native libraries", e)
        } catch (t: Throwable) {
            Log.e(TAG, "Unexpected native initialization failure", t)
        } finally {
            synchronized(loadingLock) {
                isLibLoading = false
            }
        }
    }

    /**
     * Non-blocking check: returns true only if native libraries are already loaded.
     * Does NOT wait or block. Use [awaitLibrariesLoaded] on a background coroutine if you need to wait.
     */
    fun isLoaded(): Boolean = isLibLoaded

    /**
     * Suspend until native libraries are loaded or the timeout expires.
     * Must be called from a coroutine — never from the main thread.
     */
    suspend fun awaitLibrariesLoaded(timeoutMs: Long = 10000L): Boolean {
        val deadline = System.currentTimeMillis() + timeoutMs
        while (!isLibLoaded && System.currentTimeMillis() < deadline) {
            kotlinx.coroutines.delay(50)
        }
        if (!isLibLoaded) {
            Log.e(TAG, "awaitLibrariesLoaded: timeout after ${timeoutMs}ms")
        }
        return isLibLoaded
    }

    external fun initWaylandConnection(surface: Surface, activity: Any, distroId: String): Boolean
    external fun rebindSurface(surface: Surface)
    external fun ensureSocketRuntime(context: Any, distroId: String): Boolean
    external fun onSurfaceChanged(width: Int, height: Int, physicalWidthMm: Int, physicalHeightMm: Int)
    external fun releaseWaylandConnection()
    external fun initAudioBridge()
    external fun startRecording()
    external fun stopRecording()
    external fun isRecording(): Boolean
    external fun closeAudioBridge()
    external fun initUsbRedirection(fd: Int)
    external fun closeUsbRedirection()
    external fun updateClipboard(text: String)
    external fun sendClipboardTextToWayland(text: String)
    external fun suspendRendering()
    external fun resumeRendering()
    external fun areClientsConnected(): Boolean
    external fun sendTouchEvent(action: Int, id: Int, x: Float, y: Float)
    external fun sendKeyEvent(keycode: Int, isDown: Boolean)
    external fun sendTextInput(text: String)
    external fun sendRelativeMotion(dx: Float, dy: Float, time: Int)
    external fun sendTrackpadClick(state: Int, button: Int, time: Int)
    external fun setX11SocketDir(dir: String)
    external fun notifyXwaylandReady(displayNumber: Int)
    external fun setInputMode(mode: Int)
    external fun setScrollSensitivity(value: Float)
    external fun setRelativeSensitivity(value: Float)
    external fun setRefreshRate(rate: Float)
    external fun setResolution(width: Int, height: Int)
    external fun setScale(scale: Float)
    external fun setYOffset(y_offset: Int)
    external fun getWaylandRuntimeStats(): String
    external fun getBackendSnapshot(): String
    external fun getLastNativeError(): String?
    external fun pollWaylandClipboard(): String?
    external fun getWaylandClipboardGen(): Long
    external fun pollImeVisible(): Boolean

    fun setResolutionSafe(width: Int, height: Int) {
        if (width <= 0 || height <= 0) {
            Log.w(TAG, "Ignoring invalid resolution ${width}x${height}")
            return
        }

        if (!isLibLoaded) {
            Log.w(TAG, "Native library not loaded; cannot apply resolution")
            return
        }

        runCatching { setResolution(width, height) }
            .onFailure { error ->
                Log.e(TAG, "setResolution failed for ${width}x${height}", error)
            }
    }

    fun setScaleSafe(scale: Float) {
        if (scale <= 0f) {
            Log.w(TAG, "Ignoring invalid setScale $scale")
            return
        }
        if (!isLibLoaded) {
            Log.w(TAG, "Native library not loaded; cannot apply scale")
            return
        }
        runCatching { setScale(scale) }
            .onFailure { error ->
                Log.e(TAG, "setScale failed for $scale", error)
            }
    }

    fun setClipboardListener(listener: ((String) -> Unit)?) {
        clipboardListener = listener
    }

    @JvmStatic
    fun onWaylandClipboardChanged(text: String) {
        clipboardListener?.invoke(text)
        Log.d(TAG, "Wayland clipboard changed: len=${text.length}")
    }

    @JvmStatic
    fun onWaylandShowSoftKeyboard() {
        DisplayActivity.showSoftKeyboard()
    }

    @JvmStatic
    fun onWaylandHideSoftKeyboard() {
        DisplayActivity.hideSoftKeyboard()
    }

    // ── Persistent session flag ──────────────────────────────────────────
    // Survives process kills. Written by ChrootInstaller, read by DisplayActivity.
    private const val SESSION_FLAG_NAME = ".session_active"

    private fun sessionFlagFile(context: android.content.Context): java.io.File {
        return java.io.File(context.getUnifiedFilesDir(), SESSION_FLAG_NAME)
    }

    fun markSessionActive(context: android.content.Context) {
        try {
            sessionFlagFile(context).writeText("1")
            Log.i(TAG, "Session flag: active")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to write session flag", e)
        }
    }

    fun markSessionInactive(context: android.content.Context) {
        try {
            sessionFlagFile(context).delete()
            Log.i(TAG, "Session flag: cleared")
        } catch (e: Exception) {
            Log.w(TAG, "Failed to clear session flag", e)
        }
    }

    fun wasSessionActive(context: android.content.Context): Boolean {
        return try {
            sessionFlagFile(context).readText().trim() == "1"
        } catch (_: Exception) {
            false
        }
    }
}
