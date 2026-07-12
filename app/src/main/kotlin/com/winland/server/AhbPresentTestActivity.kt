package com.winland.server

import android.app.Activity
import android.os.Bundle
import android.util.Log
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.FrameLayout
import android.widget.TextView
import kotlin.concurrent.thread

/** Isolated validation activity for the Android-owned AHardwareBuffer bridge. */
class AhbPresentTestActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var status: TextView

    companion object {
        init { System.loadLibrary("ahb_present_test") }
        @JvmStatic private external fun nativeRun(surface: Surface): Int
    }

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
        status.text = "Waiting for Turnip producer on padputer-ahb-present"
        thread(name = "ahb-present-test") {
            val result = nativeRun(holder.surface)
            Log.i("AhbPresentTest", "nativeRun result=$result")
            runOnUiThread { status.text = if (result == 0) "AHB zero-copy EGL presentation succeeded" else "Native test failed: $result" }
        }
    }
    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) = Unit
    override fun surfaceDestroyed(holder: SurfaceHolder) = Unit
}
