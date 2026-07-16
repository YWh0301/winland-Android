package com.winland.server

import android.view.Surface

/** Temporary JNI boundary for the validated Android-owned AHB presenter. */
object AhbPresenterBridge {
    init { System.loadLibrary("ahb_present_test") }

    @JvmStatic private external fun nativeRun(surface: Surface, generation: Int, width: Int, height: Int): Int
    @JvmStatic private external fun nativeSurfaceControlCursorProbe(surface: Surface): Int
    @JvmStatic private external fun nativeCreateOuterCursor(surface: Surface, generation: Int, x: Int, y: Int, visible: Boolean): Int
    @JvmStatic private external fun nativeMoveOuterCursor(generation: Int, serial: Long, x: Int, y: Int, visible: Boolean): Int
    @JvmStatic private external fun nativeDestroyOuterCursor(generation: Int): Int

    fun run(surface: Surface, generation: Int = 1, width: Int = 256, height: Int = 256): Int =
        nativeRun(surface, generation, width, height)

    fun runSurfaceControlCursorProbe(surface: Surface): Int = nativeSurfaceControlCursorProbe(surface)
    fun createOuterCursor(surface: Surface, generation: Int, x: Int, y: Int, visible: Boolean = false): Int =
        nativeCreateOuterCursor(surface, generation, x, y, visible)
    fun moveOuterCursor(generation: Int, serial: Long, x: Int, y: Int, visible: Boolean = true): Int =
        nativeMoveOuterCursor(generation, serial, x, y, visible)
    fun destroyOuterCursor(generation: Int): Int = nativeDestroyOuterCursor(generation)
}
