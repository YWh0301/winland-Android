package com.winland.server

import android.content.Context
import android.content.Intent
import android.hardware.usb.UsbManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.PowerManager
import android.os.SystemClock
import android.provider.Settings
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.lifecycleScope
import com.winland.server.ui.RootAccessRequiredDialog
import com.winland.server.ui.WinlandDashboardActions
import com.winland.server.ui.WinlandDashboardScreen
import com.winland.server.ui.theme.WinlandServerTheme
import com.winland.server.utils.RootUtils
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : ComponentActivity() {

    private val TAG = "MainActivity"

    private var serviceStarted = false
    private val uiViewModel: MainViewModel by viewModels()
    private val actionCoordinator by lazy {
        MainActionCoordinator(
            appContext = this,
            scope = lifecycleScope,
            viewModel = uiViewModel,
            showToast = ::showToastSafe,
            runtimeLauncher = RuntimeLauncher {
                startWinlandService()
                val distroId = uiViewModel.activeDistroId.value ?: "ubuntu"
                startActivity(Intent(this, DisplayActivity::class.java).apply {
                    putExtra("distro_id", distroId)
                })
            }
        )
    }

    private val usbManager by lazy {
        getSystemService(Context.USB_SERVICE) as UsbManager
    }
    private val platformController by lazy {
        MainActivityPlatformController(
            activity = this,
            usbManager = usbManager,
            showToast = ::showToastSafe
        )
    }

    private fun launchBridgeDisplay(source: Intent) {
        startActivity(Intent(this, DisplayActivity::class.java).apply {
            putExtra("bridge_only", true)
            putExtra("ahb_presenter", source.getBooleanExtra("ahb_presenter", false))
            putExtra("ahb_generation", source.getIntExtra("ahb_generation", 1))
            putExtra("ahb_next_generation", source.getIntExtra("ahb_next_generation", 0))
            putExtra("ahb_width", source.getIntExtra("ahb_width", 1696))
            putExtra("ahb_height", source.getIntExtra("ahb_height", 1200))
            putExtra("ahb_next_width", source.getIntExtra("ahb_next_width", source.getIntExtra("ahb_width", 1696)))
            putExtra("ahb_next_height", source.getIntExtra("ahb_next_height", source.getIntExtra("ahb_height", 1200)))
            putExtra("input_mode_mask", source.getIntExtra("input_mode_mask", 2))
            putExtra("input_latency_trace", source.getBooleanExtra("input_latency_trace", false))
            putExtra("outer_cursor_probe", source.getBooleanExtra("outer_cursor_probe", false))
            putExtra("distro_id", "bridge")
        })
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (!intent.getBooleanExtra("bridge_only", false)) return
        lifecycleScope.launch {
            if (NativeBridge.awaitLibrariesLoaded(30_000L)) launchBridgeDisplay(intent)
            else Log.e(TAG, "Bridge-only relaunch aborted: native libraries failed to load")
        }
    }

    private fun showToastSafe(message: String, longDuration: Boolean = false) {
        if (isFinishing || isDestroyed) return
        if (!lifecycle.currentState.isAtLeast(androidx.lifecycle.Lifecycle.State.STARTED)) return
        runOnUiThread {
            if (isFinishing || isDestroyed) return@runOnUiThread
            Toast.makeText(
                this,
                message,
                if (longDuration) Toast.LENGTH_LONG else Toast.LENGTH_SHORT
            ).show()
        }
    }

    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions()
    ) { permissions ->
        val allGranted = permissions.all { it.value }
        if (!allGranted) {
            showToastSafe("Required permissions denied")
        }
    }

    private inline fun traceLifecycle(name: String, block: () -> Unit) {
        val start = SystemClock.elapsedRealtime()
        Log.i(TAG, "$name start")
        try {
            block()
        } finally {
            val elapsed = SystemClock.elapsedRealtime() - start
            Log.i(TAG, "$name end (${elapsed}ms)")
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        traceLifecycle("onCreate") {
            super.onCreate(savedInstanceState)

            // Padputer bridge-only mode: expose only the audited Android
            // SurfaceView + Smithay parent compositor. Do not start the
            // foreground service, audio bridge, downloader, root checker, or
            // chroot manager. This mode is launched explicitly over ADB.
            if (intent.getBooleanExtra("bridge_only", false)) {
                lifecycleScope.launch {
                    if (!NativeBridge.awaitLibrariesLoaded(30_000L)) {
                        Log.e(TAG, "Bridge-only launch aborted: native libraries failed to load")
                        finish()
                        return@launch
                    }
                    launchBridgeDisplay(intent)
                    finish()
                }
                return@traceLifecycle
            }

            enableEdgeToEdge()

            if (!NativeBridge.isLoaded()) {
                showToastSafe("CRITICAL ERROR: Native libraries failed to load. The app cannot function.", longDuration = true)
            } else {
                // Keep runtime sockets (wayland/audio) available as soon as app opens.
                // Socket bootstrap moved to WinlandService (onCreate) for lifecycle ordering.
                startWinlandService()
            }

            platformController.registerUsbReceiver()

            // Request permissions after a short delay to avoid blocking the window
            // during first layout (prevents ANR from FocusEvent timeout)
            lifecycleScope.launch {
                delay(500)
                platformController.requestRequiredPermissions(permissionLauncher)
                requestBatteryOptimizationExemptionIfNeeded()
            }

            setContent {
                val themeSettings by uiViewModel.themeSettings.collectAsState()

                WinlandServerTheme(darkTheme = if (themeSettings.followSystemTheme) isSystemInDarkTheme() else themeSettings.darkModeEnabled) {
                    val scope = rememberCoroutineScope()
                    var isRootAvailable by remember { mutableStateOf(true) }
                    var showRootDialog by remember { mutableStateOf(false) }

                    LaunchedEffect(Unit) {
                        val root = withContext(Dispatchers.IO) { RootUtils.isRootAvailable() }
                        isRootAvailable = root
                        showRootDialog = !root
                    }

                    if (showRootDialog) {
                        RootAccessRequiredDialog(
                            onRetry = {
                                scope.launch {
                                    val root = withContext(Dispatchers.IO) { RootUtils.isRootAvailable() }
                                    if (root) showRootDialog = false
                                }
                            }
                        )
                    }

                    Surface(
                        modifier = Modifier.fillMaxSize(),
                        color = MaterialTheme.colorScheme.background
                    ) {
                        WinlandDashboardScreen(
                            distros = DistroCatalog.supportedDistros,
                            viewModel = uiViewModel,
                            actions = WinlandDashboardActions(
                                onRequestUsb = { platformController.requestUsbPermission() },
                                onDistroInstall = { distro -> actionCoordinator.handleDistroInstall(distro) },
                                onDistroSetup = { distroId -> actionCoordinator.handleDistroSetup(distroId) },
                                onDistroRun = { distroId -> actionCoordinator.handleDistroRun(distroId) },
                                onDistroStop = { actionCoordinator.handleDistroStop() },
                                onDistroRestart = { actionCoordinator.handleDistroRestart() },
                                onShowMessage = ::showToastSafe
                            )
                        )
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        traceLifecycle("onDestroy") {
            super.onDestroy()
            platformController.unregisterUsbReceiver()
        }
    }

    private fun startWinlandService() {
        if (serviceStarted) return
        val intent = Intent(this, WinlandService::class.java)
        startForegroundService(intent)
        serviceStarted = true
    }

    private fun requestBatteryOptimizationExemptionIfNeeded() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.M) return

        val powerManager = getSystemService(Context.POWER_SERVICE) as? PowerManager ?: return
        if (powerManager.isIgnoringBatteryOptimizations(packageName)) return

        try {
            val intent = Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS).apply {
                data = Uri.parse("package:$packageName")
            }
            startActivity(intent)
        } catch (e: Exception) {
            Log.w("MainActivity", "Failed to request battery optimization exemption", e)
        }
    }

}
