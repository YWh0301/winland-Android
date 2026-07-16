package com.winland.server

import android.view.Surface

/** Temporary JNI boundary for the validated Android-owned AHB presenter. */
object AhbPresenterBridge {
    init { System.loadLibrary("ahb_present_test") }

    @JvmStatic private external fun nativeRun(surface: Surface, generation: Int, width: Int, height: Int): Int
    @JvmStatic private external fun nativeSurfaceControlCursorProbe(surface: Surface): Int

    fun run(surface: Surface, generation: Int = 1, width: Int = 256, height: Int = 256): Int =
        nativeRun(surface, generation, width, height)

    fun runSurfaceControlCursorProbe(surface: Surface): Int = nativeSurfaceControlCursorProbe(surface)
}
