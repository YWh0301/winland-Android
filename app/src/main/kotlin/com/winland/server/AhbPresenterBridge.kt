package com.winland.server

import android.view.Surface

/** Temporary JNI boundary for the validated Android-owned AHB presenter. */
object AhbPresenterBridge {
    init { System.loadLibrary("ahb_present_test") }

    @JvmStatic private external fun nativeRun(surface: Surface, generation: Int, width: Int, height: Int): Int
    @JvmStatic private external fun nativeSurfaceControlCursorProbe(surface: Surface): Int
    @JvmStatic private external fun nativeOuterCursorImageProbe(generation: Int): Int
    @JvmStatic private external fun nativeArmOuterCursorController(generation: Int): Int
    @JvmStatic private external fun nativeRunOuterCursorController(generation: Int): Int
    @JvmStatic private external fun nativeStopOuterCursorController(generation: Int): Int
    @JvmStatic private external fun nativeCreateOuterCursor(surface: Surface, generation: Int, x: Int, y: Int, visible: Boolean, diagnosticBuffer: Boolean, outerScale: Float): Int
    @JvmStatic private external fun nativeMoveOuterCursor(generation: Int, serial: Long, x: Int, y: Int, visible: Boolean): Int
    @JvmStatic private external fun nativeDestroyOuterCursor(generation: Int): Int

    fun run(surface: Surface, generation: Int = 1, width: Int = 256, height: Int = 256): Int =
        nativeRun(surface, generation, width, height)

    fun runSurfaceControlCursorProbe(surface: Surface): Int = nativeSurfaceControlCursorProbe(surface)
    fun runOuterCursorImageProbe(generation: Int): Int = nativeOuterCursorImageProbe(generation)
    fun armOuterCursorController(generation: Int): Int = nativeArmOuterCursorController(generation)
    fun runOuterCursorController(generation: Int): Int = nativeRunOuterCursorController(generation)
    fun stopOuterCursorController(generation: Int): Int = nativeStopOuterCursorController(generation)
    fun createOuterCursor(surface: Surface, generation: Int, x: Int, y: Int, visible: Boolean = false, diagnosticBuffer: Boolean = true, outerScale: Float = 1f): Int =
        nativeCreateOuterCursor(surface, generation, x, y, visible, diagnosticBuffer, outerScale)
    fun moveOuterCursor(generation: Int, serial: Long, x: Int, y: Int, visible: Boolean = true): Int =
        nativeMoveOuterCursor(generation, serial, x, y, visible)
    fun destroyOuterCursor(generation: Int): Int = nativeDestroyOuterCursor(generation)
}
