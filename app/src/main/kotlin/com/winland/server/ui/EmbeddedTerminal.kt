package com.winland.server.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.os.Build
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.util.Log
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.inputmethod.InputMethodManager
import com.termux.terminal.TerminalEmulator
import com.termux.terminal.TerminalSession
import com.termux.terminal.TerminalSessionClient
import com.termux.view.TerminalView
import com.termux.view.TerminalViewClient
import com.winland.server.engine.ChrootInstaller
import com.winland.server.utils.getUnifiedFilesDir
import com.winland.server.utils.getUnifiedRootfsDir
import com.winland.server.utils.getUnifiedTmpDir

class EmbeddedTerminal(private val context: Context) : TerminalSessionClient, TerminalViewClient {

    companion object {
        private const val TAG = "EmbeddedTerminal"
    }

    var terminalView: TerminalView? = null
        private set
    var currentSession: TerminalSession? = null
        private set
    private var currentFontSize = 14
    private var sessionFinishedHandled = false
    private var restartCount = 0
    private val MAX_RESTARTS = 2
    private var scaleAccumulator = 0f
    private var shellPid: Int = -1
    private var currentDistroId: String = "ubuntu"
    var onSessionStateChanged: ((Boolean) -> Unit)? = null
    var barCtrlActive: Boolean = false
    var barAltActive: Boolean = false
    var onBarStateChanged: ((ctrl: Boolean, alt: Boolean) -> Unit)? = null

    @android.annotation.SuppressLint("ClickableViewAccessibility")
    fun createView(): TerminalView {
        applyColorScheme()
        val view = TerminalView(context, null)
        view.setTerminalViewClient(this)
        view.setTextSize(currentFontSize)
        view.keepScreenOn = true
        view.setBackgroundColor(0xFF282C34.toInt())
        view.isFocusable = true
        view.isFocusableInTouchMode = true
        view.setTypeface(android.graphics.Typeface.MONOSPACE)

        // Prevent Compose/ViewGroup parents from intercepting touch events meant for the
        // terminal (scroll history, pinch-to-zoom, text selection drags).
        view.setOnTouchListener { v, event ->
            when (event.actionMasked) {
                android.view.MotionEvent.ACTION_DOWN,
                android.view.MotionEvent.ACTION_POINTER_DOWN ->
                    v.parent?.requestDisallowInterceptTouchEvent(true)
                android.view.MotionEvent.ACTION_UP,
                android.view.MotionEvent.ACTION_POINTER_UP,
                android.view.MotionEvent.ACTION_CANCEL ->
                    v.parent?.requestDisallowInterceptTouchEvent(false)
            }
            false // pass through to TerminalView.onTouchEvent()
        }

        terminalView = view
        return view
    }

    private fun applyColorScheme() {
        // One Dark theme
        val props = java.util.Properties()
        props["foreground"] = "#ABB2BF"
        props["background"] = "#282C34"
        props["cursor"] = "#528BFF"
        // Normal colors
        props["color0"] = "#282C34"   // black
        props["color1"] = "#E06C75"   // red
        props["color2"] = "#98C379"   // green
        props["color3"] = "#E5C07B"   // yellow
        props["color4"] = "#61AFEF"   // blue
        props["color5"] = "#C678DD"   // magenta
        props["color6"] = "#56B6C2"   // cyan
        props["color7"] = "#ABB2BF"   // white
        // Bright colors
        props["color8"] = "#5C6370"   // bright black
        props["color9"] = "#E06C75"   // bright red
        props["color10"] = "#98C379"  // bright green
        props["color11"] = "#E5C07B"  // bright yellow
        props["color12"] = "#61AFEF"  // bright blue
        props["color13"] = "#C678DD"  // bright magenta
        props["color14"] = "#56B6C2"  // bright cyan
        props["color15"] = "#FFFFFF"  // bright white
        com.termux.terminal.TerminalColors.COLOR_SCHEME.updateWith(props)
    }

    fun startSession(distroId: String = "ubuntu") {
        currentDistroId = distroId
        val tv = terminalView ?: return
        if (currentSession != null) {
            // Re-attach existing session to the (possibly new) view
            tv.attachSession(currentSession)
            tv.requestFocus()
            return
        }

        // Use ACTUAL path for local file operations to bypass sandbox/symlink issues
        val physicalFilesDir = context.filesDir.absolutePath
        // Use UNIFIED path for strings being passed into chroot/shell scripts
        val unifiedFilesDir = context.getUnifiedFilesDir()
        
        val rootfsDir = context.getUnifiedRootfsDir(distroId)
        val status = ChrootInstaller.getChrootStatus(context, distroId)

        val shellBinary = findShellBinary()
        val isSu = shellBinary.endsWith("/su")
        Log.i(TAG, "Using shell: $shellBinary (su=$isSu, chrootReady=${status.ready})")

        val args: Array<String>
        val cwd: String

        if (status.ready && isSu) {
            val chrootScriptFile = java.io.File(physicalFilesDir, "chroot-dashboard_$distroId.sh")
            chrootScriptFile.writeText(buildChrootCommand(rootfsDir, unifiedFilesDir, distroId))
            chrootScriptFile.setExecutable(true, false)
            cwd = "/"
            args = arrayOf("su", "-c", "sh ${chrootScriptFile.absolutePath}")
        } else if (isSu) {
            cwd = "/"
            args = arrayOf("su")
        } else {
            cwd = unifiedFilesDir
            args = arrayOf("sh")
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
            tv.attachSession(session)
            tv.requestFocus()
            onSessionStateChanged?.invoke(true)

            tv.postDelayed({
                val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
                imm.showSoftInput(tv, InputMethodManager.SHOW_IMPLICIT)
            }, 500)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create terminal session", e)
        }
    }

    fun destroy() {
        currentSession?.finishIfRunning()
        currentSession = null
        terminalView = null
    }

    fun restartSession() {
        sessionFinishedHandled = true
        currentSession?.finishIfRunning()
        currentSession = null
        restartCount = 0
        sessionFinishedHandled = false
        onSessionStateChanged?.invoke(false)
        terminalView?.post { startSession(currentDistroId) }
    }

    fun increaseFontSize() {
        val newSize = (currentFontSize + 1).coerceIn(6, 48)
        if (newSize != currentFontSize) {
            currentFontSize = newSize
            terminalView?.setTextSize(currentFontSize)
        }
    }

    fun decreaseFontSize() {
        val newSize = (currentFontSize - 1).coerceIn(6, 48)
        if (newSize != currentFontSize) {
            currentFontSize = newSize
            terminalView?.setTextSize(currentFontSize)
        }
    }

    fun sendSpecialKey(key: String) {
        val session = currentSession ?: return
        when (key) {
            "ESC" -> session.write(byteArrayOf(27), 0, 1)
            "TAB" -> session.write(byteArrayOf(9), 0, 1)
            "CTRL" -> { /* handled as modifier */ }
            "ALT" -> { /* handled as modifier */ }
            "HOME" -> session.write("\u001b[H".toByteArray(), 0, 3)
            "END" -> session.write("\u001b[F".toByteArray(), 0, 3)
            "PGUP" -> session.write("\u001b[5~".toByteArray(), 0, 4)
            "PGDN" -> session.write("\u001b[6~".toByteArray(), 0, 4)
            "UP" -> session.write("\u001b[A".toByteArray(), 0, 3)
            "DOWN" -> session.write("\u001b[B".toByteArray(), 0, 3)
            "LEFT" -> session.write("\u001b[D".toByteArray(), 0, 3)
            "RIGHT" -> session.write("\u001b[C".toByteArray(), 0, 3)
            "DEL" -> session.write("\u001b[3~".toByteArray(), 0, 4)
            "/" -> session.write("/".toByteArray(), 0, 1)
            "-" -> session.write("-".toByteArray(), 0, 1)
            "|" -> session.write("|".toByteArray(), 0, 1)
            "~" -> session.write("~".toByteArray(), 0, 1)
            else -> {
                if (key.startsWith("CTRL+") && key.length == 6) {
                    val ch = key[5]
                    val code = (ch.uppercaseChar().code - 64).toByte()
                    session.write(byteArrayOf(code), 0, 1)
                }
            }
        }
    }

    private fun startFallbackSession() {
        val tv = terminalView ?: return
        val shellBinary = findShellBinary()
        val isSu = shellBinary.endsWith("/su")
        Log.i(TAG, "Starting fallback session with: $shellBinary")
        val env = arrayOf(
            "TERM=xterm-256color",
            "HOME=/root",
            "COLORTERM=truecolor",
            "LANG=en_US.UTF-8"
        )
        val args = if (isSu) arrayOf<String>() else arrayOf<String>()
        try {
            val session = TerminalSession(shellBinary, "/", args, env, null, this)
            currentSession = session
            tv.attachSession(session)
            tv.requestFocus()
        } catch (e: Exception) {
            Log.e(TAG, "Failed to create fallback session", e)
        }
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
            if (java.io.File(path).exists()) return path
        }
        return null
    }

    private fun findShellBinary(): String {
        val su = findSuBinary()
        if (su != null) return su
        val shCandidates = listOf("/system/bin/sh", "/bin/sh")
        for (path in shCandidates) {
            if (java.io.File(path).exists()) return path
        }
        return "/system/bin/sh"
    }

    private fun buildChrootCommand(rootfsDir: String, filesDir: String, distroId: String): String {
        val tmpDir = "$filesDir/tmp"
        return """#!/bin/sh
ROOTFS_DIR="$rootfsDir"
TMP_DIR="$tmpDir"
FILES_DIR="$filesDir"
EXT_STORAGE=/storage/emulated/0

unmount_safe() {
    umount "${'$'}1" 2>/dev/null || umount -l "${'$'}1" 2>/dev/null || true
}

[ ! -d "${'$'}ROOTFS_DIR" ] && exec /system/bin/sh

mkdir -p "${'$'}ROOTFS_DIR/proc" "${'$'}ROOTFS_DIR/sys" "${'$'}ROOTFS_DIR/dev" "${'$'}ROOTFS_DIR/dev/pts" "${'$'}ROOTFS_DIR/tmp" "${'$'}ROOTFS_DIR/dev/shm" "${'$'}ROOTFS_DIR/external_storage" 2>/dev/null
mkdir -p "${'$'}TMP_DIR" 2>/dev/null

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
mount -o bind "${'$'}TMP_DIR" "${'$'}ROOTFS_DIR${'$'}FILES_DIR/tmp" 2>/dev/null || true

chmod 1777 "${'$'}ROOTFS_DIR/tmp" 2>/dev/null || true
chmod 1777 "${'$'}TMP_DIR" 2>/dev/null || true
chmod 1777 "${'$'}ROOTFS_DIR/dev/shm" 2>/dev/null || true

mkdir -p "${'$'}ROOTFS_DIR${'$'}FILES_DIR/tmp" 2>/dev/null
chmod 700 "${'$'}ROOTFS_DIR${'$'}FILES_DIR/tmp" 2>/dev/null || true

# --- Socket accessibility fix (chroot needs world access) ---
SOCK="${'$'}TMP_DIR/wayland-0"
LOGF="${'$'}TMP_DIR/winland-socket-fix.log"
if [ -S "${'$'}SOCK" ]; then
  echo "[fix] Found socket: ${'$'}SOCK" >> "${'$'}LOGF"
  chmod 777 "${'$'}SOCK" 2>>"${'$'}LOGF" || echo "[fix] chmod failed: $?" >> "${'$'}LOGF"
  ls -lZ "${'$'}SOCK" >> "${'$'}LOGF"
else
  echo "[fix] Socket not found: ${'$'}SOCK" >> "${'$'}LOGF"
fi
# --- End fix ---

export HOME=/root
export USER=root
export LOGNAME=root
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export TERM=xterm-256color
export COLORTERM=truecolor
export LANG=en_US.UTF-8
export PS1='root@winland_$distroId:\w# '
export XDG_RUNTIME_DIR=${'$'}FILES_DIR/tmp
export WAYLAND_DISPLAY=wayland-0
if [ -f "${'$'}ROOTFS_DIR/bin/bash" ]; then
    exec chroot "${'$'}ROOTFS_DIR" /bin/bash --login
elif [ -f "${'$'}ROOTFS_DIR/bin/sh" ]; then
    exec chroot "${'$'}ROOTFS_DIR" /bin/sh -l
else
    echo "[!] Chroot failed: no shell found in ${'$'}ROOTFS_DIR"
    echo "[!] Falling back to Android root shell"
    exec /system/bin/sh
fi
"""
    }

    var onTitleChanged: ((String) -> Unit)? = null

    // === TerminalSessionClient ===

    override fun onTextChanged(changedSession: TerminalSession) {
        terminalView?.onScreenUpdated()
    }

    override fun onTitleChanged(changedSession: TerminalSession) {
        val title = changedSession.title
        if (!title.isNullOrEmpty()) {
            Log.d(TAG, "Terminal title changed: $title")
            onTitleChanged?.invoke(title)
        }
    }

    override fun onSessionFinished(finishedSession: TerminalSession) {
        val exitCode = finishedSession.exitStatus
        Log.w(TAG, "Session finished with exit code: $exitCode (restart count: $restartCount/$MAX_RESTARTS)")
        onSessionStateChanged?.invoke(false)
        if (sessionFinishedHandled) return
        sessionFinishedHandled = true
        if (restartCount < MAX_RESTARTS) {
            restartCount++
            terminalView?.postDelayed({
                sessionFinishedHandled = false
                currentSession = null
                startSession(currentDistroId)
            }, 1000)
        } else {
            Log.e(TAG, "Max restarts reached, not restarting")
            // Start a simple fallback shell
            terminalView?.postDelayed({
                sessionFinishedHandled = false
                currentSession = null
                startFallbackSession()
            }, 500)
        }
    }

    override fun onCopyTextToClipboard(session: TerminalSession, text: String) {
        try {
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            clipboard.setPrimaryClip(ClipData.newPlainText("Terminal", text))
        } catch (e: Exception) {
            android.util.Log.w("EmbeddedTerminal", "Clipboard copy denied (not in focus)", e)
        }
    }

    override fun onPasteTextFromClipboard(session: TerminalSession?) {
        try {
            val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
            val clip = clipboard.primaryClip
            if (clip != null && clip.itemCount > 0) {
                val text = clip.getItemAt(0).coerceToText(context).toString()
                currentSession?.emulator?.paste(text)
            }
        } catch (e: Exception) {
            android.util.Log.w("EmbeddedTerminal", "Clipboard paste denied (not in focus)", e)
        }
    }

    override fun onBell(session: TerminalSession) {
        try {
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val vm = context.getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as? VibratorManager
                vm?.defaultVibrator?.vibrate(VibrationEffect.createOneShot(50, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                val v = context.getSystemService(Context.VIBRATOR_SERVICE) as? Vibrator
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
        applyColorScheme()
        terminalView?.invalidate()
    }

    override fun onTerminalCursorStateChange(state: Boolean) {
        Log.d(TAG, "Terminal cursor state changed: visible=$state")
        terminalView?.invalidate()
    }

    override fun setTerminalShellPid(session: TerminalSession, pid: Int) {
        shellPid = pid
        Log.i(TAG, "Terminal shell PID set: $pid")
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
        scaleAccumulator += (scale - 1f)
        if (scaleAccumulator > 0.3f) {
            scaleAccumulator = 0f
            val newSize = (currentFontSize + 1).coerceIn(6, 48)
            if (newSize != currentFontSize) {
                currentFontSize = newSize
                terminalView?.setTextSize(currentFontSize)
            }
        } else if (scaleAccumulator < -0.3f) {
            scaleAccumulator = 0f
            val newSize = (currentFontSize - 1).coerceIn(6, 48)
            if (newSize != currentFontSize) {
                currentFontSize = newSize
                terminalView?.setTextSize(currentFontSize)
            }
        }
        return scale
    }

    override fun onSingleTapUp(e: MotionEvent) {
        val imm = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        imm.showSoftInput(terminalView, InputMethodManager.SHOW_IMPLICIT)
    }

    override fun shouldBackButtonBeMappedToEscape(): Boolean = false
    override fun shouldEnforceCharBasedInput(): Boolean = true
    override fun shouldUseCtrlSpaceWorkaround(): Boolean = false
    override fun isTerminalViewSelected(): Boolean = true
    override fun copyModeChanged(copyMode: Boolean) {
        Log.d(TAG, "Copy mode: $copyMode")
    }

    override fun onKeyDown(keyCode: Int, e: KeyEvent, session: TerminalSession): Boolean {
        if (keyCode == KeyEvent.KEYCODE_ENTER || keyCode == KeyEvent.KEYCODE_DEL) {
            if (barCtrlActive) {
                barCtrlActive = false
                onBarStateChanged?.invoke(false, barAltActive)
            }
        }
        return false
    }

    override fun onKeyUp(keyCode: Int, e: KeyEvent): Boolean {
        if (barCtrlActive && keyCode != KeyEvent.KEYCODE_CTRL_LEFT && keyCode != KeyEvent.KEYCODE_CTRL_RIGHT) {
            barCtrlActive = false
            onBarStateChanged?.invoke(false, barAltActive)
        }
        if (barAltActive && keyCode != KeyEvent.KEYCODE_ALT_LEFT && keyCode != KeyEvent.KEYCODE_ALT_RIGHT) {
            barAltActive = false
            onBarStateChanged?.invoke(barCtrlActive, false)
        }
        return false
    }

    override fun onLongPress(event: MotionEvent): Boolean {
        return false
    }

    override fun readControlKey(): Boolean = barCtrlActive
    override fun readAltKey(): Boolean = barAltActive
    override fun readShiftKey(): Boolean = false
    override fun readFnKey(): Boolean = false

    override fun onCodePoint(codePoint: Int, ctrlDown: Boolean, session: TerminalSession): Boolean {
        if (ctrlDown && barCtrlActive) {
            barCtrlActive = false
            onBarStateChanged?.invoke(false, barAltActive)
        }
        if (barAltActive) {
            barAltActive = false
            onBarStateChanged?.invoke(barCtrlActive, false)
        }
        return false
    }
    override fun onEmulatorSet() {
        terminalView?.post { terminalView?.requestFocus() }
    }
}
