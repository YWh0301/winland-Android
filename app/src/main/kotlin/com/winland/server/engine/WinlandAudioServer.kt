package com.winland.server.engine

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.util.Log
import kotlinx.coroutines.*
import java.io.File
import java.io.FileInputStream
import java.io.FileNotFoundException

class WinlandAudioServer {
    private val TAG = "WinlandAudioServer"
    private var audioTrack: AudioTrack? = null
    private var isRunning = false
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var serverJob: Job? = null

    companion object {
        private const val FIFO_PATH = "/data/data/com.winland.server/files/tmp/audio_bridge/fifo"
    }

    fun start() {
        if (isRunning) {
            Log.d(TAG, "Audio server already running")
            return
        }

        isRunning = true
        initAudioTrack()
        val fifoDir = File(FIFO_PATH).parentFile
        if (fifoDir != null && !fifoDir.exists()) {
            fifoDir.mkdirs()
            Log.i(TAG, "Created FIFO directory: $fifoDir")
        }
        serverJob = scope.launch {
            while (isActive && isRunning) {
                try {
                    val fifoFile = File(FIFO_PATH)
                    if (!fifoFile.exists()) {
                        Log.d(TAG, "Waiting for FIFO $FIFO_PATH...")
                        delay(1000)
                        continue
                    }
                    Log.i(TAG, "Opening FIFO: $FIFO_PATH")
                    val inputStream = FileInputStream(fifoFile)
                    Log.i(TAG, "FIFO opened, audio bridge connected")
                    val buffer = ByteArray(8192)
                    try {
                        while (isRunning) {
                            val bytesRead = inputStream.read(buffer)
                            if (bytesRead == -1) break
                            if (bytesRead > 0) {
                                audioTrack?.write(buffer, 0, bytesRead)
                            }
                        }
                    } catch (e: Exception) {
                        if (isRunning) {
                            Log.e(TAG, "FIFO read error", e)
                        }
                    } finally {
                        try { inputStream.close() } catch (_: Exception) {}
                    }
                    Log.i(TAG, "FIFO read ended, will retry")
                } catch (e: FileNotFoundException) {
                    if (isRunning) {
                        delay(1000)
                    }
                } catch (e: Exception) {
                    if (isRunning) {
                        Log.e(TAG, "Audio server error; retrying in 1s", e)
                        delay(1000)
                    }
                }
            }
        }
    }

    private fun initAudioTrack() {
        val minBufferSize = AudioTrack.getMinBufferSize(
            44100,
            AudioFormat.CHANNEL_OUT_STEREO,
            AudioFormat.ENCODING_PCM_16BIT
        )
        audioTrack = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(44100)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                    .build()
            )
            .setBufferSizeInBytes(minBufferSize)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()
        audioTrack?.play()
    }

    fun stop() {
        isRunning = false
        serverJob?.cancel()
        serverJob = null
        try { audioTrack?.stop() } catch (_: Exception) {}
        try { audioTrack?.release() } catch (_: Exception) {}
        audioTrack = null
    }
}
