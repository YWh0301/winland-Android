package com.winland.server

import android.app.Application
import android.app.ActivityManager
import android.content.Context
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.winland.server.NativeBridge
import com.winland.server.engine.ChrootInstaller
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class MainViewModel(application: Application) : AndroidViewModel(application) {

    private data class CpuSnapshot(val total: Long, val idle: Long)

    data class DisplayInfo(
        val logicalW: Int = 0,
        val logicalH: Int = 0,
        val physicalW: Int = 0,
        val physicalH: Int = 0,
        val scaleW: Float = 1f,
        val scaleH: Float = 1f,
        val windowBound: Boolean = false,
        val shmEnabled: Boolean = false,
        val windows: Int = 0,
        val unmanaged: Int = 0,
        val fps: Float = 0f
    )

    data class SystemTelemetry(
        val cpuUsage: Float = 0f,
        val ramUsage: Float = 0f,
        val gpuUsage: Float? = null,
        val cpuHistory: List<Float> = List(24) { 0f },
        val ramHistory: List<Float> = List(24) { 0f },
        val gpuHistory: List<Float> = List(24) { 0f }
    )

    data class DistroUiState(
        val progress: Float = 0f,
        val isDownloading: Boolean = false,
        val isRunLaunching: Boolean = false,
        val stageText: String = "Idle",
        val lastStageUpdate: String = "--:--:--"
    )

    data class ChrootRuntimeState(
        val installed: Boolean = false,
        val ready: Boolean = false,
        val reason: String = "Not installed or incomplete rootfs",
        val isExtracted: Boolean = false
    )

    enum class UiOperation {
        DOWNLOAD,
        SETUP,
        RUN
    }

    enum class InputMode(val value: Int) {
        Touch(1),
        Trackpad(2),
        Mouse(4);

        companion object {
            fun fromValue(v: Int): InputMode = entries.find { it.value == v } ?: Touch
        }
    }

    data class ThemeSettings(
        val followSystemTheme: Boolean,
        val darkModeEnabled: Boolean,
        val screenPreset: String
    )

    private val prefs = application.getSharedPreferences("winland_prefs", Context.MODE_PRIVATE)

    private val _themeSettings = MutableStateFlow(readThemeSettings())
    val themeSettings: StateFlow<ThemeSettings> = _themeSettings.asStateFlow()

    private val _logSearchQuery = MutableStateFlow("")
    val logSearchQuery: StateFlow<String> = _logSearchQuery.asStateFlow()

    private val _rawLogs = MutableStateFlow<List<String>>(emptyList())
    private val _filteredLogs = MutableStateFlow<List<String>>(emptyList())
    val filteredLogs: StateFlow<List<String>> = _filteredLogs.asStateFlow()

    private val _activeUiOperation = MutableStateFlow<UiOperation?>(null)
    val activeUiOperation: StateFlow<UiOperation?> = _activeUiOperation.asStateFlow()

    private val _selectedTab = MutableStateFlow(DashboardTab.Home)
    val selectedTab: StateFlow<DashboardTab> = _selectedTab.asStateFlow()

    private val _logsPaused = MutableStateFlow(false)
    val logsPaused: StateFlow<Boolean> = _logsPaused.asStateFlow()

    private val _inputMode = MutableStateFlow(InputMode.fromValue(prefs.getInt("input_mode_mask", 1)))
    val inputMode: StateFlow<InputMode> = _inputMode.asStateFlow()

    fun updateInputMode(mode: InputMode) {
        _inputMode.value = mode
        prefs.edit().putInt("input_mode_mask", mode.value).apply()
        if (NativeBridge.isLoaded()) {
            NativeBridge.setInputMode(mode.value)
        }
    }

    private val _terminalInput = MutableStateFlow("")
    val terminalInput: StateFlow<String> = _terminalInput.asStateFlow()

    private val _terminalLines = MutableStateFlow<List<String>>(emptyList())
    val terminalLines: StateFlow<List<String>> = _terminalLines.asStateFlow()

    private val _terminalHistory = MutableStateFlow<List<String>>(emptyList())
    val terminalHistory: StateFlow<List<String>> = _terminalHistory.asStateFlow()

    private val _showDebugOverlay = MutableStateFlow(false)
    val showDebugOverlay: StateFlow<Boolean> = _showDebugOverlay.asStateFlow()

    private val _telemetry = MutableStateFlow(SystemTelemetry())
    val telemetry: StateFlow<SystemTelemetry> = _telemetry.asStateFlow()

    private val _displayInfo = MutableStateFlow(DisplayInfo())
    val displayInfo: StateFlow<DisplayInfo> = _displayInfo.asStateFlow()

    private val _distroUiStates = MutableStateFlow<Map<String, DistroUiState>>(emptyMap())
    val distroUiStates: StateFlow<Map<String, DistroUiState>> = _distroUiStates.asStateFlow()

    private val _activeDistroId = MutableStateFlow(prefs.getString("active_distro_id", null))
    val activeDistroId: StateFlow<String?> = _activeDistroId.asStateFlow()

    private val _chrootRuntimeState = MutableStateFlow(ChrootRuntimeState())
    val chrootRuntimeState: StateFlow<ChrootRuntimeState> = _chrootRuntimeState.asStateFlow()

    private val _perDistroChrootState = MutableStateFlow<Map<String, ChrootRuntimeState>>(emptyMap())
    val perDistroChrootState: StateFlow<Map<String, ChrootRuntimeState>> = _perDistroChrootState.asStateFlow()

    private val _perDistroRefreshSignal = MutableStateFlow(0)
    val perDistroRefreshSignal: StateFlow<Int> = _perDistroRefreshSignal.asStateFlow()

    private var previousCpuSnapshot = readCpuSnapshot()
    private var hasReportedGpuUnsupported = false

    init {
        // Defer chroot status check to background thread to avoid blocking ViewModel init
        // Start with default "checking" state, update asynchronously
        viewModelScope.launch(Dispatchers.IO) {
            refreshChrootRuntimeState()
        }
        startTelemetryLoop()
    }

    private fun readThemeSettings(): ThemeSettings {
        return ThemeSettings(
            followSystemTheme = prefs.getBoolean("theme_follow_system", true),
            darkModeEnabled = prefs.getBoolean("theme_dark_enabled", true),
            screenPreset = prefs.getString("screen_preset", "Standard") ?: "Standard"
        )
    }

    fun updateThemeMode(followSystem: Boolean, darkEnabled: Boolean) {
        prefs.edit()
            .putBoolean("theme_follow_system", followSystem)
            .putBoolean("theme_dark_enabled", darkEnabled)
            .apply()
        _themeSettings.value = _themeSettings.value.copy(
            followSystemTheme = followSystem,
            darkModeEnabled = darkEnabled
        )
    }

    fun updateScreenPreset(preset: String) {
        prefs.edit().putString("screen_preset", preset).apply()
        _themeSettings.value = _themeSettings.value.copy(screenPreset = preset)
    }

    fun setLogSearchQuery(query: String) {
        _logSearchQuery.value = query
        recomputeFilteredLogs()
    }

    fun appendLogLine(line: String) {
        _rawLogs.value = (_rawLogs.value + line).takeLast(250)
        recomputeFilteredLogs()
    }

    fun setSelectedTab(tab: DashboardTab) {
        _selectedTab.value = tab
    }

    fun setLogsPaused(paused: Boolean) {
        _logsPaused.value = paused
    }

    fun toggleLogsPaused() {
        _logsPaused.value = !_logsPaused.value
    }

    fun setTerminalInput(value: String) {
        _terminalInput.value = value
    }

    fun appendTerminalLine(line: String) {
        _terminalLines.value = (_terminalLines.value + line).takeLast(500)
    }

    fun clearTerminalLines() {
        _terminalLines.value = emptyList()
    }

    fun recordTerminalCommand(command: String) {
        val normalized = command.trim()
        if (normalized.isEmpty()) return
        _terminalHistory.value = listOf(normalized) + _terminalHistory.value.filterNot { it == normalized }.take(7)
    }

    fun setShowDebugOverlay(visible: Boolean) {
        _showDebugOverlay.value = visible
    }

    fun toggleDebugOverlay() {
        _showDebugOverlay.value = !_showDebugOverlay.value
    }

    fun ensureDistroStates(ids: List<String>) {
        val current = _distroUiStates.value.toMutableMap()
        var changed = false
        ids.forEach { id ->
            if (!current.containsKey(id)) {
                current[id] = DistroUiState()
                changed = true
            }
        }
        if (changed) {
            _distroUiStates.value = current
        }
    }

    fun setDistroProgress(id: String, progress: Float) {
        updateDistroState(id) { it.copy(progress = progress) }
    }

    fun setDistroDownloading(id: String, downloading: Boolean) {
        updateDistroState(id) { it.copy(isDownloading = downloading) }
    }

    fun setDistroRunLaunching(id: String, launching: Boolean) {
        updateDistroState(id) { it.copy(isRunLaunching = launching) }
    }

    fun setActiveDistro(id: String?) {
        prefs.edit().putString("active_distro_id", id).apply()
        _activeDistroId.value = id
    }

    fun updateDistroStage(id: String, stage: String) {
        updateDistroState(id) {
            it.copy(
                stageText = stage,
                lastStageUpdate = nowStamp()
            )
        }
    }

    fun touchDistroUpdate(id: String) {
        updateDistroState(id) {
            it.copy(lastStageUpdate = nowStamp())
        }
    }

    @Synchronized
    fun tryBeginOperation(operation: UiOperation): Boolean {
        if (_activeUiOperation.value != null) return false
        _activeUiOperation.value = operation
        return true
    }

    @Synchronized
    fun endOperation(operation: UiOperation) {
        if (_activeUiOperation.value == operation) {
            _activeUiOperation.value = null
        }
    }

    private fun recomputeFilteredLogs() {
        val query = _logSearchQuery.value.trim()
        val logs = _rawLogs.value
        _filteredLogs.value = if (query.isBlank()) {
            logs
        } else {
            logs.filter { it.contains(query, ignoreCase = true) }
        }
    }

    private fun startTelemetryLoop() {
        viewModelScope.launch(Dispatchers.IO) {
            while (true) {
                val nextCpu = readCpuSnapshot()
                val cpu = cpuPercent(previousCpuSnapshot, nextCpu)
                previousCpuSnapshot = nextCpu
                val ram = getRamUsagePercent(getApplication())
                val gpu = readGpuUsagePercent()

                if (gpu == null && !hasReportedGpuUnsupported) {
                    appendLogLine("GPU telemetry unavailable on this device/kernel")
                    hasReportedGpuUnsupported = true
                } else if (gpu != null && hasReportedGpuUnsupported) {
                    appendLogLine("GPU telemetry is now available")
                    hasReportedGpuUnsupported = false
                }

                val prev = _telemetry.value
                _telemetry.value = prev.copy(
                    cpuUsage = cpu,
                    ramUsage = ram,
                    gpuUsage = gpu,
                    cpuHistory = (prev.cpuHistory + cpu).takeLast(24),
                    ramHistory = (prev.ramHistory + ram).takeLast(24),
                    gpuHistory = (prev.gpuHistory + (gpu ?: prev.gpuHistory.lastOrNull() ?: 0f)).takeLast(24)
                )

                val backendSnapshot = runCatching { NativeBridge.getBackendSnapshot() }.getOrNull() ?: ""
                _displayInfo.value = parseDisplayInfo(backendSnapshot)

                delay(1_000)
            }
        }
    }

    fun refreshChrootRuntimeState() {
        val context = getApplication<Application>()
        val distroId = _activeDistroId.value ?: "ubuntu"
        val status = ChrootInstaller.getChrootStatus(context, distroId)
        _chrootRuntimeState.value = ChrootRuntimeState(
            installed = status.installed,
            ready = status.ready,
            reason = status.reason,
            isExtracted = status.extracted
        )
        refreshAllPerDistroStates()
    }

    fun refreshChrootRuntimeStateForDistro(distroId: String) {
        val context = getApplication<Application>()
        val status = ChrootInstaller.getChrootStatus(context, distroId)
        val state = ChrootRuntimeState(
            installed = status.installed,
            ready = status.ready,
            reason = status.reason,
            isExtracted = status.extracted
        )
        _perDistroChrootState.value = _perDistroChrootState.value + (distroId to state)
    }

    fun refreshAllPerDistroStates() {
        val context = getApplication<Application>()
        val knownIds = _distroUiStates.value.keys
        if (knownIds.isEmpty()) return
        val newStates = knownIds.associateWith { id ->
            val status = ChrootInstaller.getChrootStatus(context, id)
            ChrootRuntimeState(
                installed = status.installed,
                ready = status.ready,
                reason = status.reason,
                isExtracted = status.extracted
            )
        }
        _perDistroChrootState.value = newStates
        _perDistroRefreshSignal.value++
    }

    private fun updateDistroState(id: String, transform: (DistroUiState) -> DistroUiState) {
        val current = _distroUiStates.value.toMutableMap()
        val state = current[id] ?: DistroUiState()
        current[id] = transform(state)
        _distroUiStates.value = current
    }

    private fun nowStamp(): String = SimpleDateFormat("HH:mm:ss", Locale.US).format(Date())

    private fun readCpuSnapshot(): CpuSnapshot {
        return try {
            val cpuLine = File("/proc/stat").useLines { lines ->
                lines.firstOrNull { it.startsWith("cpu ") }
            } ?: return CpuSnapshot(0L, 0L)

            val parts = cpuLine.trim().split(Regex("\\s+"))
            if (parts.size < 8) return CpuSnapshot(0L, 0L)

            val user = parts.getOrNull(1)?.toLongOrNull() ?: 0L
            val nice = parts.getOrNull(2)?.toLongOrNull() ?: 0L
            val system = parts.getOrNull(3)?.toLongOrNull() ?: 0L
            val idle = parts.getOrNull(4)?.toLongOrNull() ?: 0L
            val iowait = parts.getOrNull(5)?.toLongOrNull() ?: 0L
            val irq = parts.getOrNull(6)?.toLongOrNull() ?: 0L
            val softirq = parts.getOrNull(7)?.toLongOrNull() ?: 0L

            CpuSnapshot(
                total = user + nice + system + idle + iowait + irq + softirq,
                idle = idle + iowait
            )
        } catch (_: Exception) {
            CpuSnapshot(0L, 0L)
        }
    }

    private fun cpuPercent(previous: CpuSnapshot, current: CpuSnapshot): Float {
        val totalDelta = (current.total - previous.total).coerceAtLeast(1L)
        val idleDelta = (current.idle - previous.idle).coerceAtLeast(0L)
        val busy = (totalDelta - idleDelta).coerceAtLeast(0L)
        return (busy.toFloat() / totalDelta.toFloat() * 100f).coerceIn(0f, 100f)
    }

    private fun getRamUsagePercent(context: Context): Float {
        return try {
            val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
            val memInfo = ActivityManager.MemoryInfo()
            activityManager.getMemoryInfo(memInfo)
            val total = memInfo.totalMem.toFloat()
            if (total <= 0f) return 0f
            val used = total - memInfo.availMem.toFloat()
            (used / total * 100f).coerceIn(0f, 100f)
        } catch (_: Exception) {
            0f
        }
    }

    private fun readGpuUsagePercent(): Float? {
        val candidates = listOf(
            "/sys/class/kgsl/kgsl-3d0/gpubusy",
            "/sys/class/devfreq/gpu/load",
            "/sys/class/misc/mali0/device/utilization"
        )

        for (path in candidates) {
            val value = readSingleLine(path) ?: continue
            parseGpuUsage(value)?.let { return it }
        }

        return null
    }

    private fun parseDisplayInfo(snapshot: String): DisplayInfo {
        val logical = Regex("""logical=(\d+)x(\d+)""").find(snapshot)
        val physical = Regex("""physical=(\d+)x(\d+)""").find(snapshot)
        val scale = Regex("""scale=([\d.]+)x([\d.]+)""").find(snapshot)
        val windowBound = snapshot.contains("window_bound=true")
        val shmEnabled = snapshot.contains("shm_enabled=true")
        return DisplayInfo(
            logicalW = logical?.groupValues?.get(1)?.toIntOrNull() ?: 0,
            logicalH = logical?.groupValues?.get(2)?.toIntOrNull() ?: 0,
            physicalW = physical?.groupValues?.get(1)?.toIntOrNull() ?: 0,
            physicalH = physical?.groupValues?.get(2)?.toIntOrNull() ?: 0,
            scaleW = scale?.groupValues?.get(1)?.toFloatOrNull() ?: 1f,
            scaleH = scale?.groupValues?.get(2)?.toFloatOrNull() ?: 1f,
            windowBound = windowBound,
            shmEnabled = shmEnabled
        )
    }

    private fun readSingleLine(path: String): String? {
        return try {
            File(path).bufferedReader().use { it.readLine()?.trim() }
        } catch (_: Exception) {
            null
        }
    }

    private fun parseGpuUsage(raw: String): Float? {
        if (raw.isBlank()) return null

        val parts = raw.split(Regex("\\s+"))

        // Adreno kgsl: "busy total"
        if (parts.size >= 2) {
            val busy = parts[0].toLongOrNull()
            val total = parts[1].toLongOrNull()
            if (busy != null && total != null && total > 0L) {
                return ((busy.toFloat() / total.toFloat()) * 100f).coerceIn(0f, 100f)
            }
        }

        // Generic devfreq or mali style: single number (0..100 or 0..1000)
        val single = parts.firstOrNull()?.toFloatOrNull() ?: return null
        return when {
            single <= 100f -> single
            single <= 1000f -> single / 10f
            else -> null
        }?.coerceIn(0f, 100f)
    }
}
