package com.winland.server

import android.view.Surface

/** Temporary JNI boundary for the validated Android-owned AHB presenter. */
object AhbPresenterBridge {
    init { System.loadLibrary("ahb_present_test") }

    @JvmStatic private external fun nativeRun(surface: Surface, generation: Int): Int

    fun run(surface: Surface, generation: Int = 1): Int = nativeRun(surface, generation)
}
