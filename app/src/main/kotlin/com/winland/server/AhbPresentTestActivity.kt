package com.winland.server

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.FrameLayout
import android.widget.TextView
import kotlin.concurrent.thread

/** Isolated validation activity for the Android-owned AHardwareBuffer bridge. */
class AhbPresentTestActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var status: TextView

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val root = FrameLayout(this)
        val surface = SurfaceView(this)
        status = TextView(this).apply {
            text = "Waiting for Android surface"
            textSize = 18f
            setTextColor(0xffffffff.toInt())
            setBackgroundColor(0x88000000.toInt())
            setPadding(24, 24, 24, 24)
        }
        root.addView(surface, FrameLayout.LayoutParams(-1, -1))
        root.addView(status, FrameLayout.LayoutParams(-1, -2))
        setContentView(root)
        surface.holder.addCallback(this)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        val cursorProbe = intent.getBooleanExtra("surface_control_cursor_probe", false)
        status.text = if (cursorProbe) "Probing app-owned SurfaceControl cursor layer" else "Waiting for Turnip producer on padputer-ahb-present"
        thread(name = "ahb-present-test") {
            val result = if (cursorProbe) AhbPresenterBridge.runSurfaceControlCursorProbe(holder.surface) else AhbPresenterBridge.run(holder.surface)
            Log.i("AhbPresentTest", "nativeRun cursorProbe=$cursorProbe result=$result")
            runOnUiThread {
                status.text = when {
                    result != 0 -> "Native test failed: $result"
                    cursorProbe -> "SurfaceControl cursor layer succeeded"
                    else -> "AHB zero-copy EGL presentation succeeded"
                }
            }
        }
    }
    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) = Unit
    override fun surfaceDestroyed(holder: SurfaceHolder) = Unit
}
