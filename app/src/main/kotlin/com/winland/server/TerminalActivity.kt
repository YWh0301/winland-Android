package com.winland.server

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Color
import android.os.Build
import android.os.Bundle
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.util.Log
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsetsController
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import android.widget.Toast
import androidx.activity.ComponentActivity
import com.termux.terminal.TerminalEmulator
import com.termux.terminal.TerminalSession
import com.termux.terminal.TerminalSessionClient
import com.termux.view.TerminalView
import com.termux.view.TerminalViewClient
import com.winland.server.engine.ChrootInstaller
import com.winland.server.utils.*

class TerminalActivity : ComponentActivity(), TerminalSessionClient, TerminalViewClient {

    companion object {
        private const val TAG = "TerminalActivity"
    }

    private lateinit var terminalView: TerminalView
    private var currentSession: TerminalSession? = null
    private var currentFontSize = 14
    private var sessionFinishedHandled = false
    private var shellPid: Int = -1

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Fullscreen immersive
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.R) {
            window.insetsController?.let {
                it.hide(android.view.WindowInsets.Type.statusBars() or android.view.WindowInsets.Type.navigationBars())
                it.systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            }
        }

        // Create terminal view
        terminalView = TerminalView(this, null)
        terminalView.setTerminalViewClient(this)
        terminalView.setTextSize(currentFontSize)
        terminalView.keepScreenOn = true
        terminalView.setBackgroundColor(Color.BLACK)

        val container = FrameLayout(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(terminalView, FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ))
        }
        setContentView(container)

        startTerminalSession()
    }

    private fun findSuBinary(): String? {
        val candidates = listOf(
            "/sbin/su",
            "/system/bin/su",
            "/system/xbin/su",
            "/su/bin/su",
            "/magisk/.core/bin/su",
            "/data/adb/su/bin/su"
        )
        for (path in candidates) {
            if (java.io.File(path).exists()) {
                Log.i(TAG, "Found su at: $path")
                return path
            }
        }
        return null
    }

    private fun findShellBinary(): String {
        // Try su first, fall back to sh
        val su = findSuBinary()
        if (su != null) return su

        val shCandidates = listOf("/system/bin/sh", "/bin/sh")
        for (path in shCandidates) {
            if (java.io.File(path).exists()) {
                Log.w(TAG, "su not found, falling back to: $path")
                return path
            }
        }
        return "/system/bin/sh" // last resort
    }

    private fun startTerminalSession() {
        val distroId = intent.getStringExtra("distro_id") ?: "ubuntu"
        val filesDir = getUnifiedFilesDir()
        val rootfsDir = getUnifiedRootfsDir(distroId)
        val status = ChrootInstaller.getChrootStatus(this, distroId)

        val shellBinary = findShellBinary()
        val isSu = shellBinary.endsWith("/su")
        Log.i(TAG, "Using shell: $shellBinary (su=$isSu, chrootReady=${status.ready})")

        val args: Array<String>
        val cwd: String

        if (status.ready && isSu) {
            // Write chroot script to a file and execute it via su
            val chrootScriptFile = java.io.File(filesDir, "chroot-terminal_$distroId.sh")
            chrootScriptFile.writeText(buildChrootCommand(rootfsDir, filesDir, distroId))
            chrootScriptFile.setExecutable(true, false)
            cwd = "/"
            args = arrayOf("-c", "sh ${chrootScriptFile.absolutePath}")
        } else if (isSu) {
            cwd = "/"
            args = arrayOf()
        } else {
            cwd = filesDir
            args = arrayOf()
        }
        val env = arrayOf(
            "TERM=xterm-256color",
            "HOME=/root",
            "COLORTERM=truecolor",
            "LANG=en_US.UTF-8"
        )

        try {
            val session = TerminalSession(shellBinary, cwd, args, env, null, this)
            currentSession = session
            terminalView.attachSession(session)
            terminalView.requestFocus()

            // Show keyboard after layout
            terminalView.postDelayed({
                val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                imm.showSoftInput(terminalView, InputMethodManager.SHOW_IMPLICIT)
            }, 500)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create terminal session", e)
            Toast.makeText(this, "Failed to start terminal: ${e.message}", Toast.LENGTH_LONG).show()
        }
    }

    private fun buildChrootCommand(rootfsDir: String, filesDir: String, distroId: String): String {
        val tmpDir = "$filesDir/tmp"
        return """
            set -e
            ROOTFS_DIR="$rootfsDir"
            TMP_DIR="$tmpDir"
            FILES_DIR="$filesDir"
            EXT_STORAGE=/storage/emulated/0

            unmount_safe() {
                umount "$1" 2>/dev/null || umount -l "$1" 2>/dev/null || true
            }

            if [ ! -d "${'$'}ROOTFS_DIR" ]; then
                echo "ERROR: rootfs missing: ${'$'}ROOTFS_DIR"
                exec /system/bin/sh
            fi

            mkdir -p "${'$'}ROOTFS_DIR/proc" "${'$'}ROOTFS_DIR/sys" "${'$'}ROOTFS_DIR/dev" "${'$'}ROOTFS_DIR/dev/pts" "${'$'}ROOTFS_DIR/tmp" "${'$'}ROOTFS_DIR/dev/shm" "${'$'}ROOTFS_DIR/external_storage"
            mkdir -p "${'$'}TMP_DIR"

            mount -o remount,dev,suid /data 2>/dev/null || true

            unmount_safe "${'$'}ROOTFS_DIR/dev/shm"
            unmount_safe "${'$'}ROOTFS_DIR/external_storage"
            unmount_safe "${'$'}ROOTFS_DIR/tmp"
            unmount_safe "${'$'}ROOTFS_DIR/dev/pts"
            unmount_safe "${'$'}ROOTFS_DIR/dev"
            unmount_safe "${'$'}ROOTFS_DIR/proc"
            unmount_safe "${'$'}ROOTFS_DIR/sys"

            mount -t proc proc "${'$'}ROOTFS_DIR/proc" 2>/dev/null || true
            mount -t sysfs sys "${'$'}ROOTFS_DIR/sys" 2>/dev/null || true
            mount -o bind /dev "${'$'}ROOTFS_DIR/dev" 2>/dev/null || true
            mount -o bind /dev/pts "${'$'}ROOTFS_DIR/dev/pts" 2>/dev/null || true
            mount -o bind "${'$'}EXT_STORAGE" "${'$'}ROOTFS_DIR/external_storage" 2>/dev/null || true
            mount -t tmpfs -o nosuid,nodev tmpfs "${'$'}ROOTFS_DIR/dev/shm" 2>/dev/null || true
            mount -o bind "${'$'}TMP_DIR" "${'$'}ROOTFS_DIR/tmp" 2>/dev/null || true
            mkdir -p "${'$'}ROOTFS_DIR${'$'}FILES_DIR/tmp"
            mount -o bind "${'$'}TMP_DIR" "${'$'}ROOTFS_DIR${'$'}FILES_DIR/tmp" 2>/dev/null || true

chmod 1777 "${'$'}ROOTFS_DIR/tmp" 2>/dev/null || true
chmod 1777 "${'$'}TMP_DIR" 2>/dev/null || true

export HOME=/root
            export USER=root
            export TERM=xterm-256color
            export COLORTERM=truecolor
            export LANG=en_US.UTF-8
            export PS1='root@winland_$distroId:\w# '
            export XDG_RUNTIME_DIR=${'$'}FILES_DIR/tmp
            export WAYLAND_DISPLAY=wayland-0
            exec chroot "${'$'}ROOTFS_DIR" /bin/bash --login
        """.trimIndent()
    }

    // === TerminalSessionClient ===

    override fun onTextChanged(changedSession: TerminalSession) {
        terminalView.onScreenUpdated()
    }

    override fun onTitleChanged(changedSession: TerminalSession) {
        val title = changedSession.title
        if (!title.isNullOrEmpty()) {
            runOnUiThread { setTitle("Winland › $title") }
        }
    }

    override fun onSessionFinished(finishedSession: TerminalSession) {
        if (sessionFinishedHandled) return
        sessionFinishedHandled = true
        runOnUiThread {
            Log.w(TAG, "Terminal session finished. Shell PID was: ${finishedSession.pid}")
            // Don't auto-close — restart a basic shell so user can debug
            Toast.makeText(this, "Session ended. Restarting shell...", Toast.LENGTH_SHORT).show()
            terminalView.postDelayed({
                sessionFinishedHandled = false
                startTerminalSession()
            }, 500)
        }
    }

    override fun onCopyTextToClipboard(session: TerminalSession, text: String) {
        try {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("Terminal", text))
            Toast.makeText(this, "Copied", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Log.w(TAG, "Clipboard copy denied (not in focus)", e)
        }
    }

    override fun onPasteTextFromClipboard(session: TerminalSession?) {
        try {
            val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = clipboard.primaryClip
            if (clip != null && clip.itemCount > 0) {
                val text = clip.getItemAt(0).coerceToText(this).toString()
                currentSession?.emulator?.paste(text)
            }
        } catch (e: Exception) {
            Log.w(TAG, "Clipboard paste denied (not in focus)", e)
        }
    }

    override fun onBell(session: TerminalSession) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val vm = getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager
                vm?.defaultVibrator?.vibrate(VibrationEffect.createOneShot(50, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                val v = getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    v?.vibrate(VibrationEffect.createOneShot(50, VibrationEffect.DEFAULT_AMPLITUDE))
                } else {
                    @Suppress("DEPRECATION")
                    v?.vibrate(50)
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Bell vibration failed", e)
        }
    }

    override fun onColorsChanged(session: TerminalSession) {
        terminalView.invalidate()
    }

    override fun onTerminalCursorStateChange(state: Boolean) {
        terminalView.invalidate()
    }

    override fun setTerminalShellPid(session: TerminalSession, pid: Int) {
        shellPid = pid
        Log.i(TAG, "Terminal shell PID: $pid")
    }

    override fun getTerminalCursorStyle(): Int = TerminalEmulator.TERMINAL_CURSOR_STYLE_UNDERLINE

    override fun logError(tag: String, message: String) { Log.e(tag, message) }
    override fun logWarn(tag: String, message: String) { Log.w(tag, message) }
    override fun logInfo(tag: String, message: String) { Log.i(tag, message) }
    override fun logDebug(tag: String, message: String) { Log.d(tag, message) }
    override fun logVerbose(tag: String, message: String) { Log.v(tag, message) }
    override fun logStackTraceWithMessage(tag: String, message: String, e: Exception) { Log.e(tag, message, e) }
    override fun logStackTrace(tag: String, e: Exception) { Log.e(tag, "Exception", e) }

    // === TerminalViewClient ===

    override fun onScale(scale: Float): Float {
        if (scale < 0.9f || scale > 1.1f) {
            val newSize = if (scale > 1f) currentFontSize + 1 else currentFontSize - 1
            currentFontSize = newSize.coerceIn(8, 32)
            terminalView.setTextSize(currentFontSize)
        }
        return scale
    }

    override fun onSingleTapUp(e: MotionEvent) {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.showSoftInput(terminalView, InputMethodManager.SHOW_IMPLICIT)
    }

    override fun shouldBackButtonBeMappedToEscape(): Boolean = false
    override fun shouldEnforceCharBasedInput(): Boolean = true
    override fun shouldUseCtrlSpaceWorkaround(): Boolean = false
    override fun isTerminalViewSelected(): Boolean = true
    override fun copyModeChanged(copyMode: Boolean) {
        if (copyMode) {
            Toast.makeText(this, "Text selection active — long-press to paste", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onKeyDown(keyCode: Int, e: KeyEvent, session: TerminalSession): Boolean = false
    override fun onKeyUp(keyCode: Int, e: KeyEvent): Boolean = false

    override fun onLongPress(event: MotionEvent): Boolean {
        // Long press to paste
        onPasteTextFromClipboard(currentSession)
        return true
    }

    override fun readControlKey(): Boolean = false
    override fun readAltKey(): Boolean = false
    override fun readShiftKey(): Boolean = false
    override fun readFnKey(): Boolean = false

    override fun onCodePoint(codePoint: Int, ctrlDown: Boolean, session: TerminalSession): Boolean = false
    override fun onEmulatorSet() {
        terminalView.post { terminalView.requestFocus() }
    }

    override fun onDestroy() {
        super.onDestroy()
        currentSession?.finishIfRunning()
    }
}
