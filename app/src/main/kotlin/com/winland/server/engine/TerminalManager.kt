package com.winland.server.engine

import android.content.Context
import android.util.Log
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import java.io.File
import java.util.concurrent.TimeUnit
import com.winland.server.utils.*

class TerminalManager {
    private val TAG = "TerminalManager"
    private var session: RootCommandRunner.InteractiveSession? = null
    private var inChrootSession = false
    private var appContext: Context? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val sessionMutex = Mutex()

    private val _outputFlow = MutableSharedFlow<String>(replay = 100)
    val outputFlow = _outputFlow.asSharedFlow()

    private suspend fun ensureSession(): RootCommandRunner.InteractiveSession? {
        return sessionMutex.withLock {
            session?.let { return it }

            try {
                val createdSession = RootCommandRunner.InteractiveSession.create(
                    onStdout = { line -> _outputFlow.tryEmit(line) },
                    onStderr = { line -> _outputFlow.tryEmit(line) },
                    onExit = {
                        inChrootSession = false
                        session = null
                    }
                )
                session = createdSession
                createdSession
            } catch (e: Exception) {
                Log.e(TAG, "Terminal failed to start", e)
                null
            }
        }
    }

    fun start() {
        scope.launch {
            ensureSession()
        }
    }

    fun sendCommand(command: String) {
        scope.launch {
            val activeSession = ensureSession()
            if (activeSession == null) {
                return@launch
            }

            if (!inChrootSession && command.trim() != "exit") {
                if (!enterChroot(context = appContext, activeSession = activeSession)) {
                    return@launch
                }
            }

            activeSession.send(command)

            if (command.trim() == "exit") {
                inChrootSession = false
            }
        }
    }

    fun ensureChrootSession(context: Context, distroId: String = "ubuntu") {
        appContext = context.applicationContext
        scope.launch {
            val activeSession = ensureSession()
            if (activeSession == null) {
                return@launch
            }

            if (ChrootInstaller.isInstallInProgress()) {
                return@launch
            }

            if (inChrootSession) {
                return@launch
            }

            val status = ChrootInstaller.getChrootStatus(context, distroId)

            if (!status.extracted || !status.ready) {
                return@launch
            }

            enterChroot(context, activeSession, distroId)
        }
    }

    private suspend fun enterChroot(
        context: Context?,
        activeSession: RootCommandRunner.InteractiveSession,
        distroId: String = "ubuntu"
    ): Boolean {
        if (inChrootSession) {
            return true
        }

        val filesDir = context?.getUnifiedFilesDir() ?: "/data/user/0/com.winland.server/files"
        val rootfsDir = context?.getUnifiedRootfsDir(distroId) ?: "$filesDir/rootfs_$distroId"
        val tmpDir = context?.getUnifiedTmpDir() ?: "$filesDir/tmp"
        val scriptPath = "$filesDir/chroot-enter_$distroId.sh"

        val script = """#!/bin/sh
ROOTFS_DIR="$rootfsDir"
TMP_DIR="$tmpDir"
FILES_DIR="$filesDir"
EXT_STORAGE=/storage/emulated/0

unmount_safe() {
    umount "${'$'}1" 2>/dev/null || umount -l "${'$'}1" 2>/dev/null || true
}

[ ! -d "${'$'}ROOTFS_DIR" ] && exit 1

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

export HOME=/root
export USER=root
export LOGNAME=root
export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export TERM=xterm-256color
export PS1='root@winland_$distroId:\w# '
export XDG_RUNTIME_DIR=${'$'}FILES_DIR/tmp
export WAYLAND_DISPLAY=wayland-0
exec chroot "${'$'}ROOTFS_DIR" /bin/bash --login
"""

        try {
            val scriptFile = File(scriptPath)
            scriptFile.writeText(script)
            scriptFile.setExecutable(true)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to write chroot script", e)
            return false
        }

        // Run the chroot script in background and capture its runtime log.
        // We background the script so the interactive session remains usable.
        val bgCommand = "sh $scriptPath > $tmpDir/chroot-run.log 2>&1 &"
        try {
            activeSession.send(bgCommand)
        } catch (e: Exception) {
            Log.e(TAG, "Failed to start chroot script in interactive session", e)
            return false
        }

        // Poll for the chroot-run.log to appear, indicating startup progress.
        val maxChecks = 15
        var started = false
        for (i in 0 until maxChecks) {
            try {
                val exists = RootCommandRunner.executeBlocking("test -f $tmpDir/chroot-run.log && echo ok", 5, TimeUnit.SECONDS)
                if (exists) {
                    started = true
                    break
                }
            } catch (e: Exception) {
                Log.w(TAG, "Chroot log check failed on attempt $i: ${e.message}")
            }
            delay(1000)
        }

        if (!started) {
            Log.e(TAG, "Chroot did not start within timeout; check chroot-run.log for errors")
            return false
        }

        inChrootSession = true
        // Fetch and emit the last lines of chroot-run.log to the terminal output flow for debugging
        try {
            RootCommandRunner.execute(
                command = "tail -n 200 $tmpDir/chroot-run.log",
                timeout = 10,
                timeUnit = TimeUnit.SECONDS,
                onStdout = { line -> _outputFlow.tryEmit(line) },
                onStderr = { line -> _outputFlow.tryEmit("ERROR: $line") }
            )
        } catch (e: Exception) {
            Log.w(TAG, "Failed to fetch chroot-run.log tail: ${e.message}")
        }

        return true
    }

    fun stop() {
        session?.close()
        session = null
        inChrootSession = false
        scope.cancel()
    }
}
