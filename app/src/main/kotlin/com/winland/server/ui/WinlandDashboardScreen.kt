package com.winland.server.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CenterAlignedTopAppBar
import androidx.compose.material3.Divider
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.winland.server.DashboardTab
import com.winland.server.LinuxDistro
import com.winland.server.MainViewModel
import com.winland.server.NativeBridge
import com.winland.server.engine.ChrootInstaller

data class WinlandDashboardActions(
    val onRequestUsb: () -> Unit,
    val onDistroInstall: (LinuxDistro) -> Unit,
    val onDistroSetup: (String) -> Unit,
    val onDistroRun: (String) -> Unit,
    val onDistroStop: () -> Unit,
    val onDistroRestart: () -> Unit,
    val onShowMessage: (String, Boolean) -> Unit
)

private data class ResolutionOption(
    val label: String,
    val scale: Float
)

@Composable
fun WinlandDashboardScreen(
    distros: List<LinuxDistro>,
    viewModel: MainViewModel,
    actions: WinlandDashboardActions
) {
    val appContext = LocalContext.current
    val clipboard = LocalClipboardManager.current
    val haptics = LocalHapticFeedback.current
    val themeSettings by viewModel.themeSettings.collectAsState()
    val activeUiOperation by viewModel.activeUiOperation.collectAsState()
    val selectedTab by viewModel.selectedTab.collectAsState()
    val activeDistroId by viewModel.activeDistroId.collectAsState()
    val logsPaused by viewModel.logsPaused.collectAsState()
    val logSearchQuery by viewModel.logSearchQuery.collectAsState()
    val displayedLogs by viewModel.filteredLogs.collectAsState()

    LaunchedEffect(distros) {
        viewModel.ensureDistroStates(distros.map { it.id })
    }

    LaunchedEffect(Unit) {
        ChrootInstaller.logFlow.collect { line ->
            if (!logsPaused) {
                viewModel.appendLogLine(line)
            }
        }
    }

    val activeOperationText = when {
        activeUiOperation == MainViewModel.UiOperation.DOWNLOAD -> "Download + Extract running"
        activeUiOperation == MainViewModel.UiOperation.SETUP -> "Setup running"
        activeUiOperation == MainViewModel.UiOperation.RUN -> "Run launching"
        else -> null
    }

    Scaffold(
        topBar = { ProfessionalTopBar(activeOperationText = activeOperationText) },
        bottomBar = {
            ModernNavigationBar(
                selectedTab = selectedTab,
                onTabSelected = {
                    haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                    viewModel.setSelectedTab(it)
                }
            )
        }
    ) { innerPadding ->
        BoxWithConstraints(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            val isWide = maxWidth >= 1000.dp

            Box(modifier = Modifier.fillMaxSize()) {
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(horizontal = 14.dp)
                ) {
                    activeOperationText?.let { op ->
                        ElevatedCard(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(top = 10.dp, bottom = 6.dp),
                            colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.tertiaryContainer.copy(alpha = 0.55f))
                        ) {
                            Text(
                                text = "Active operation: $op. Buttons are locked until completion.",
                                modifier = Modifier.padding(horizontal = 12.dp, vertical = 10.dp),
                                color = MaterialTheme.colorScheme.onTertiaryContainer,
                                style = MaterialTheme.typography.labelLarge
                            )
                        }
                    }

                    Spacer(Modifier.height(6.dp))

                    if (selectedTab == DashboardTab.Home) {
                        if (isWide) {
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .weight(1f),
                                horizontalArrangement = Arrangement.spacedBy(12.dp)
                            ) {
                                LogPanel(
                                    displayedLogs = displayedLogs,
                                    logSearchQuery = logSearchQuery,
                                    logsPaused = logsPaused,
                                    onCopyLogs = { clipboard.setText(AnnotatedString(displayedLogs.joinToString("\n"))) },
                                    onToggleLogsPaused = { viewModel.toggleLogsPaused() },
                                    onSearchChange = { viewModel.setLogSearchQuery(it) },
                                    modifier = Modifier.weight(1.4f)
                                )

                                LazyColumn(
                                    modifier = Modifier.weight(1f),
                                    verticalArrangement = Arrangement.spacedBy(12.dp)
                                ) {
                                    items(distros) { distro ->
                                        DistroCard(distro, viewModel, actions)
                                    }
                                }
                            }
                        } else {
                            LazyColumn(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .weight(1f),
                                verticalArrangement = Arrangement.spacedBy(12.dp)
                            ) {
                                items(distros) { distro ->
                                    DistroCard(distro, viewModel, actions)
                                }

                                item {
                                    LogPanel(
                                        displayedLogs = displayedLogs,
                                        logSearchQuery = logSearchQuery,
                                        logsPaused = logsPaused,
                                        onCopyLogs = { clipboard.setText(AnnotatedString(displayedLogs.joinToString("\n"))) },
                                        onToggleLogsPaused = { viewModel.toggleLogsPaused() },
                                        onSearchChange = { viewModel.setLogSearchQuery(it) },
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .height(280.dp)
                                    )
                                }
                            }
                        }
                    } else if (selectedTab == DashboardTab.Terminal) {
                        val embeddedTerminal = remember { EmbeddedTerminal(appContext) }
                        var isKeyboardVisible by remember { mutableStateOf(false) }
                        var ctrlActive by remember { mutableStateOf(false) }
                        var altActive by remember { mutableStateOf(false) }
                        var sessionAlive by remember { mutableStateOf(false) }
                        var terminalTitle by remember { mutableStateOf("") }
                        val currentDistroId = activeDistroId ?: "ubuntu"

                        // Wire callbacks on each recomposition (safe – just sets lambdas)
                        embeddedTerminal.onSessionStateChanged = { alive -> sessionAlive = alive }
                        embeddedTerminal.onTitleChanged = { title -> terminalTitle = title }

                        DisposableEffect(Unit) {
                            val activity = appContext as? android.app.Activity
                            val rootView = activity?.window?.decorView
                            val listener = rootView?.let { rv ->
                                android.view.ViewTreeObserver.OnGlobalLayoutListener {
                                    val rect = android.graphics.Rect()
                                    rv.getWindowVisibleDisplayFrame(rect)
                                    val screenHeight = rv.rootView.height
                                    val keypadHeight = screenHeight - rect.bottom
                                    isKeyboardVisible = keypadHeight > 200
                                }.also { l -> rv.viewTreeObserver.addOnGlobalLayoutListener(l) }
                            }
                            onDispose {
                                if (listener != null && rootView != null) {
                                    rootView.viewTreeObserver.removeOnGlobalLayoutListener(listener)
                                }
                                embeddedTerminal.destroy()
                            }
                        }

                        Column(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(horizontal = 8.dp, vertical = 6.dp)
                                .clip(RoundedCornerShape(14.dp))
                                .background(Color(0xFF282C34))
                        ) {
                            // Terminal header
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .background(Color(0xFF21252B))
                                    .padding(horizontal = 8.dp, vertical = 4.dp),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    modifier = Modifier.weight(1f)
                                ) {
                                    Box(
                                        modifier = Modifier
                                            .size(8.dp)
                                            .clip(CircleShape)
                                            .background(
                                                if (sessionAlive) Color(0xFF98C379)
                                                else Color(0xFFE06C75)
                                            )
                                    )
                                    Spacer(modifier = Modifier.width(6.dp))
                                    Text(
                                        text = if (terminalTitle.isNotEmpty()) terminalTitle else currentDistroId,
                                        color = Color(0xFF9DA5B4),
                                        fontSize = 12.sp,
                                        fontFamily = FontFamily.Monospace,
                                        fontWeight = FontWeight.Medium,
                                        maxLines = 1,
                                        overflow = TextOverflow.Ellipsis
                                    )
                                }
                                // Header controls: font size, restart
                                Row(
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(2.dp)
                                ) {
                                    Surface(
                                        onClick = { embeddedTerminal.decreaseFontSize() },
                                        shape = RoundedCornerShape(4.dp),
                                        color = Color(0xFF3E4451),
                                        modifier = Modifier.size(width = 28.dp, height = 22.dp)
                                    ) {
                                        Box(contentAlignment = Alignment.Center) {
                                            Text(
                                                text = "A-",
                                                color = Color(0xFF5C6370),
                                                fontSize = 10.sp,
                                                fontFamily = FontFamily.Monospace
                                            )
                                        }
                                    }
                                    Surface(
                                        onClick = { embeddedTerminal.increaseFontSize() },
                                        shape = RoundedCornerShape(4.dp),
                                        color = Color(0xFF3E4451),
                                        modifier = Modifier.size(width = 28.dp, height = 22.dp)
                                    ) {
                                        Box(contentAlignment = Alignment.Center) {
                                            Text(
                                                text = "A+",
                                                color = Color(0xFF5C6370),
                                                fontSize = 10.sp,
                                                fontFamily = FontFamily.Monospace
                                            )
                                        }
                                    }
                                    IconButton(
                                        onClick = { embeddedTerminal.restartSession() },
                                        modifier = Modifier.size(28.dp)
                                    ) {
                                        Icon(
                                            imageVector = Icons.Default.Refresh,
                                            contentDescription = "Restart session",
                                            tint = Color(0xFF5C6370),
                                            modifier = Modifier.size(14.dp)
                                        )
                                    }
                                }
                            }

                            // Terminal view
                            AndroidView(
                                factory = { _ ->
                                    val view = embeddedTerminal.createView()
                                    embeddedTerminal.startSession(currentDistroId)
                                    view
                                },
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .weight(1f)
                            )

                            // Extra keys bar (shown when soft keyboard is visible)
                            if (isKeyboardVisible) {
                                TerminalExtraKeysBar(
                                    ctrlActive = ctrlActive,
                                    altActive = altActive,
                                    onCtrlToggle = { ctrlActive = !ctrlActive },
                                    onAltToggle = { altActive = !altActive },
                                    onKey = { key ->
                                        if (ctrlActive && key.length == 1 && key[0].isLetter()) {
                                            embeddedTerminal.sendSpecialKey("CTRL+${key[0]}")
                                            ctrlActive = false
                                        } else if (altActive && key.length == 1) {
                                            val esc = byteArrayOf(27)
                                            embeddedTerminal.currentSession?.write(esc, 0, 1)
                                            val ch = key.toByteArray()
                                            embeddedTerminal.currentSession?.write(ch, 0, ch.size)
                                            altActive = false
                                        } else {
                                            embeddedTerminal.sendSpecialKey(key)
                                        }
                                    }
                                )
                            }
                        }
                    } else {
                        SettingsPanel(
                            viewModel = viewModel,
                            followSystemTheme = themeSettings.followSystemTheme,
                            darkModeEnabled = themeSettings.darkModeEnabled,
                            screenPreset = themeSettings.screenPreset,
                            onThemeModeChanged = { followSystem, darkEnabled ->
                                viewModel.updateThemeMode(followSystem, darkEnabled)
                            },
                            onScreenPresetChanged = { preset ->
                                viewModel.updateScreenPreset(preset)
                                actions.onShowMessage("Screen preset set to $preset", false)
                            },
                            onResolutionApplied = { resolution ->
                                actions.onShowMessage("Resolution updated to $resolution", false)
                            },
                            onRequestUsb = actions.onRequestUsb,
                            onStopChroot = actions.onDistroStop,
                            onRestartChroot = actions.onDistroRestart,
                            activeDistroId = activeDistroId,
                            controlsEnabled = activeUiOperation == null,
                            modifier = Modifier
                                .fillMaxWidth()
                                .weight(1f)
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun LogPanel(
    displayedLogs: List<String>,
    logSearchQuery: String,
    logsPaused: Boolean,
    onCopyLogs: () -> Unit,
    onToggleLogsPaused: () -> Unit,
    onSearchChange: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    ElevatedCard(
        modifier = modifier,
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)
    ) {
        Column(modifier = Modifier.fillMaxSize().padding(10.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text("Live Logs", fontWeight = FontWeight.SemiBold)
                Spacer(Modifier.weight(1f))
                IconButton(onClick = onCopyLogs) {
                    Icon(Icons.Default.ContentCopy, "Copy logs")
                }
                IconButton(onClick = onToggleLogsPaused) {
                    Icon(if (logsPaused) Icons.Default.PlayArrow else Icons.Default.Pause, "Pause logs")
                }
            }

            OutlinedTextField(
                value = logSearchQuery,
                onValueChange = onSearchChange,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                textStyle = LocalTextStyle.current.copy(fontSize = 12.sp),
                placeholder = { Text("Filter logs...", fontSize = 12.sp) },
                trailingIcon = { Icon(Icons.Default.Search, "Search logs") }
            )

            val verticalScroll = rememberScrollState()
            val horizontalScroll = rememberScrollState()
            SelectionContainer {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(top = 8.dp)
                        .clip(RoundedCornerShape(14.dp))
                        .background(MaterialTheme.colorScheme.surface)
                        .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(14.dp))
                        .padding(10.dp)
                        .verticalScroll(verticalScroll)
                        .horizontalScroll(horizontalScroll)
                ) {
                    Text(
                        text = if (displayedLogs.isEmpty()) "No logs yet" else displayedLogs.joinToString("\n"),
                        color = MaterialTheme.colorScheme.onSurface,
                        fontFamily = FontFamily.Monospace,
                        fontSize = 12.sp,
                        lineHeight = 16.sp
                    )
                }
            }
        }
    }
}

@Composable
private fun SettingsPanel(
    viewModel: MainViewModel,
    followSystemTheme: Boolean,
    darkModeEnabled: Boolean,
    screenPreset: String,
    onThemeModeChanged: (Boolean, Boolean) -> Unit,
    onScreenPresetChanged: (String) -> Unit,
    onResolutionApplied: (String) -> Unit,
    onRequestUsb: () -> Unit,
    onStopChroot: () -> Unit,
    onRestartChroot: () -> Unit,
    activeDistroId: String?,
    controlsEnabled: Boolean,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val displayInfo by viewModel.displayInfo.collectAsState()
    var localFollowSystem by remember { mutableStateOf(followSystemTheme) }
    var localDarkMode by remember { mutableStateOf(darkModeEnabled) }
    var localScreenPreset by remember { mutableStateOf(screenPreset) }

    LaunchedEffect(followSystemTheme, darkModeEnabled, screenPreset) {
        localFollowSystem = followSystemTheme
        localDarkMode = darkModeEnabled
        localScreenPreset = screenPreset
    }

    val resolution1080p by remember {
        derivedStateOf {
            ResolutionOption("1080p", 1.0f)
        }
    }
    val resolution720p by remember {
        derivedStateOf {
            ResolutionOption("720p", 1.5f)
        }
    }
    var selectedResolutionLabel by rememberSaveable { mutableStateOf(resolution1080p.label) }
    val selectedResolution = if (selectedResolutionLabel == resolution720p.label) resolution720p else resolution1080p

    val scroll = rememberScrollState()

    Column(
        modifier = modifier
            .verticalScroll(scroll)
            .padding(horizontal = 4.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp)
    ) {
        Text(
            text = "Settings",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 6.dp)
        )

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Display", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)

                listOf(resolution1080p, resolution720p).forEach { option ->
                    Surface(
                        onClick = {
                            selectedResolutionLabel = option.label
                            NativeBridge.setScaleSafe(option.scale)
                            onResolutionApplied("${option.label}: scale=${option.scale}")
                        },
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surface,
                        tonalElevation = 1.dp,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = option.label,
                                style = MaterialTheme.typography.bodyLarge,
                                fontWeight = FontWeight.Medium
                            )
                            Switch(
                                checked = selectedResolution.label == option.label,
                                onCheckedChange = {
                                    selectedResolutionLabel = option.label
                                    NativeBridge.setScaleSafe(option.scale)
                                    onResolutionApplied("${option.label}: scale=${option.scale}")
                                }
                            )
                        }
                    }
                }
            }
        }


        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Display Info", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)

                Row(modifier = Modifier.fillMaxWidth()) {
                    Text("Window Bound", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(
                        if (displayInfo.windowBound) "Yes" else "No",
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (displayInfo.windowBound) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
                    )
                }
                Row(modifier = Modifier.fillMaxWidth()) {
                    Text("Logical (native)", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(
                        "${displayInfo.logicalW} x ${displayInfo.logicalH}",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
                Row(modifier = Modifier.fillMaxWidth()) {
                    Text("Physical (viewport)", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(
                        "${displayInfo.physicalW} x ${displayInfo.physicalH}",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
                Row(modifier = Modifier.fillMaxWidth()) {
                    Text("Scale", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(
                        "${"%.2f".format(displayInfo.scaleW)} x ${"%.2f".format(displayInfo.scaleH)}",
                        style = MaterialTheme.typography.bodyMedium
                    )
                }
                Row(modifier = Modifier.fillMaxWidth()) {
                    Text("SHM", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                    Text(
                        if (displayInfo.shmEnabled) "Enabled" else "Disabled",
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (displayInfo.shmEnabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
                    )
                }
            }
        }

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Appearance", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)

                Row(
                    modifier = Modifier.horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    FilterChip(
                        selected = localFollowSystem,
                        onClick = {
                            localFollowSystem = true
                            onThemeModeChanged(true, localDarkMode)
                        },
                        label = { Text("System") }
                    )
                    FilterChip(
                        selected = !localFollowSystem && localDarkMode,
                        onClick = {
                            localFollowSystem = false
                            localDarkMode = true
                            onThemeModeChanged(false, true)
                        },
                        label = { Text("Dark") }
                    )
                    FilterChip(
                        selected = !localFollowSystem && !localDarkMode,
                        onClick = {
                            localFollowSystem = false
                            localDarkMode = false
                            onThemeModeChanged(false, false)
                        },
                        label = { Text("Light") }
                    )
                }

                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            if (localFollowSystem) "Theme follows system" else "Dark Mode",
                            style = MaterialTheme.typography.bodyLarge
                        )
                        Text(
                            if (localFollowSystem) "Turn off System mode to control manually" else "Reduce eye strain in low light",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                    Switch(
                        checked = localDarkMode,
                        enabled = !localFollowSystem,
                        onCheckedChange = {
                            localFollowSystem = false
                            localDarkMode = it
                            onThemeModeChanged(false, it)
                        }
                    )
                }
            }
        }

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text("Runtime Controls", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                val runtimeReady = activeDistroId != null
                val buttonsEnabled = controlsEnabled && runtimeReady

                Text(
                    text = if (runtimeReady) "Active distro: $activeDistroId" else "No active distro. Install and setup first.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Row(horizontalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxWidth()) {
                    OutlinedButton(
                        onClick = onStopChroot,
                        enabled = buttonsEnabled,
                        modifier = Modifier.weight(1f).heightIn(min = 52.dp)
                    ) {
                        Text("Stop")
                    }
                    Button(
                        onClick = onRestartChroot,
                        enabled = buttonsEnabled,
                        modifier = Modifier.weight(1f).heightIn(min = 52.dp)
                    ) {
                        Text("Restart")
                    }
                }
                OutlinedButton(
                    onClick = onRequestUsb,
                    enabled = controlsEnabled,
                    modifier = Modifier.fillMaxWidth().heightIn(min = 52.dp)
                ) {
                    Icon(Icons.Default.Usb, contentDescription = null)
                    Spacer(Modifier.width(8.dp))
                    Text("USB")
                }
            }
        }

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Input Mode", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)

                val currentMode by viewModel.inputMode.collectAsState()

                MainViewModel.InputMode.entries.forEach { mode ->
                    Surface(
                        onClick = { viewModel.updateInputMode(mode) },
                        shape = RoundedCornerShape(12.dp),
                        color = MaterialTheme.colorScheme.surface,
                        tonalElevation = 1.dp,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween
                        ) {
                            Text(
                                text = "${mode.name} Mode",
                                style = MaterialTheme.typography.bodyLarge,
                                fontWeight = FontWeight.Medium
                            )
                            Switch(
                                checked = currentMode == mode,
                                onCheckedChange = { if (it) viewModel.updateInputMode(mode) }
                            )
                        }
                    }
                }
            }
        }

        Spacer(Modifier.height(20.dp))
    }
}
@Composable
private fun DistroCard(
    distro: LinuxDistro,
    viewModel: MainViewModel,
    actions: WinlandDashboardActions
) {
    val haptics = LocalHapticFeedback.current
    val activeUiOperation by viewModel.activeUiOperation.collectAsState()
    val chrootRuntimeState by viewModel.chrootRuntimeState.collectAsState()
    val activeDistroId by viewModel.activeDistroId.collectAsState()
    val isSettingUp = activeUiOperation == MainViewModel.UiOperation.SETUP
    val distroUiStates by viewModel.distroUiStates.collectAsState()
    val currentUiState = distroUiStates[distro.id] ?: MainViewModel.DistroUiState()
    val progress = currentUiState.progress
    val isDownloading = currentUiState.isDownloading
    val isRunLaunching = currentUiState.isRunLaunching
    val stageText = currentUiState.stageText
    val lastStageUpdate = currentUiState.lastStageUpdate
    val updateStage: (String) -> Unit = { next -> viewModel.updateDistroStage(distro.id, next) }

    LaunchedEffect(Unit) {
        viewModel.refreshChrootRuntimeState()
    }

    val cardOwnsRuntime = activeDistroId == distro.id
    val currentStage = when {
        cardOwnsRuntime && chrootRuntimeState.ready -> "run"
        cardOwnsRuntime && chrootRuntimeState.isExtracted -> "setup"
        else -> "install"
    }
    val statusReady = cardOwnsRuntime && chrootRuntimeState.ready
    val statusText = when {
        cardOwnsRuntime -> chrootRuntimeState.reason
        activeDistroId == null -> "No distro installed yet"
        else -> "Active distro is $activeDistroId"
    }

    val operationLocked = activeUiOperation != null
    val lockReason = when {
        activeUiOperation == MainViewModel.UiOperation.DOWNLOAD -> "Download in progress"
        activeUiOperation == MainViewModel.UiOperation.SETUP -> "Setup in progress"
        activeUiOperation == MainViewModel.UiOperation.RUN -> "Run in progress"
        else -> null
    }

    LaunchedEffect(currentStage, isDownloading, isSettingUp, isRunLaunching) {
        val nextStage = when {
            isDownloading -> "Downloading and extracting rootfs"
            isSettingUp -> "Setup in progress (installing packages)"
            isRunLaunching -> "Run launching"
            currentStage == "setup" -> "Ready for setup"
            currentStage == "run" -> "Ready to run"
            else -> "Waiting for archive download"
        }
        if (stageText != nextStage) {
            updateStage(nextStage)
        }
    }

    LaunchedEffect(isSettingUp) {
        if (!isSettingUp) return@LaunchedEffect
        ChrootInstaller.logFlow.collect {
            viewModel.touchDistroUpdate(distro.id)
        }
    }

    ElevatedCard(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.elevatedCardColors(
            containerColor = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f)
        )
    ) {
        Column(modifier = Modifier.padding(20.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(distro.name, style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                    Text(distro.description, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                StatusBadge(statusReady)
            }

            Spacer(Modifier.height(8.dp))
            Text(
                "Status: $statusText",
                style = MaterialTheme.typography.labelMedium,
                color = if (statusReady) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.tertiary
            )
            Text(
                "Phase: $stageText  |  Last update: $lastStageUpdate",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant
            )

            if (isDownloading) {
                LinearProgressIndicator(
                    progress = { progress },
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .height(6.dp)
                        .clip(CircleShape),
                    color = MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.primary.copy(alpha = 0.1f)
                )
            } else if (isSettingUp) {
                LinearProgressIndicator(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(vertical = 8.dp)
                        .height(6.dp)
                        .clip(CircleShape)
                )
            }

            if (operationLocked) {
                Text(
                    text = "Controls locked: ${lockReason ?: "Operation running"}",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.tertiary
                )
            }

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                if (currentStage != "run") {
                    Button(
                        onClick = {
                            haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                            when (currentStage) {
                                "install" -> {
                                    if (operationLocked) {
                                        actions.onShowMessage("Another operation is running. Wait until it finishes.", false)
                                        return@Button
                                    }
                                    actions.onDistroInstall(distro)
                                }

                                "setup" -> {
                                    if (operationLocked || isSettingUp) {
                                        actions.onShowMessage("Another operation is running. Wait until it finishes.", false)
                                        return@Button
                                    }
                                    actions.onDistroSetup(distro.id)
                                }

                                else -> Unit
                            }
                        },
                        enabled = !operationLocked && !isDownloading && !isSettingUp && !isRunLaunching
                    ) {
                        Icon(Icons.Default.Download, null)
                        Spacer(Modifier.width(6.dp))
                        Text(
                            when {
                                isDownloading -> "Downloading..."
                                operationLocked && currentStage == "install" -> "Install Locked"
                                operationLocked && currentStage == "setup" -> "Setup Locked"
                                isRunLaunching -> "Launching..."
                                isSettingUp -> "Setup..."
                                currentStage == "setup" -> "Setup"
                                else -> "Install"
                            }
                        )
                    }
                } else {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton(onClick = actions.onDistroStop) {
                            Text("Stop")
                        }

                        OutlinedButton(onClick = actions.onDistroRestart) {
                            Text("Restart")
                        }

                        Button(
                            onClick = {
                                haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                                if (operationLocked) {
                                    actions.onShowMessage("Download/Setup is running. Wait before Run.", false)
                                    return@Button
                                }
                                if (isRunLaunching) {
                                    actions.onShowMessage("Run is already in progress", false)
                                    return@Button
                                }
                                actions.onDistroRun(distro.id)
                            },
                            enabled = !operationLocked && !isRunLaunching && !isDownloading && !isSettingUp,
                            colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary)
                        ) {
                            Icon(Icons.Default.PlayArrow, null)
                            Spacer(Modifier.width(6.dp))
                            Text(if (isRunLaunching) "Launching..." else "Run")
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun StatusBadge(ready: Boolean) {
    Surface(
        color = if (ready) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(50)
    ) {
        Text(
            text = if (ready) "READY" else "PENDING",
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 4.dp),
            style = MaterialTheme.typography.labelSmall,
            color = if (ready) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
            fontWeight = FontWeight.Bold
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProfessionalTopBar(activeOperationText: String?) {
    CenterAlignedTopAppBar(
        title = {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text("Winland", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                if (activeOperationText != null) {
                    Text(
                        activeOperationText,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                }
            }
        },
        colors = TopAppBarDefaults.centerAlignedTopAppBarColors(
            containerColor = MaterialTheme.colorScheme.surface.copy(alpha = 0.92f)
        )
    )
}

@Composable
private fun ModernNavigationBar(selectedTab: DashboardTab, onTabSelected: (DashboardTab) -> Unit) {
    Surface(
        modifier = Modifier
            .padding(horizontal = 24.dp, vertical = 12.dp)
            .height(64.dp)
            .fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface.copy(alpha = 0.95f),
        shape = CircleShape,
        shadowElevation = 12.dp
    ) {
        Row(
            modifier = Modifier.fillMaxSize(),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically
        ) {
            val tabs = listOf(
                DashboardTab.Home to Icons.Default.Home,
                DashboardTab.Terminal to Icons.Default.Terminal,
                DashboardTab.Settings to Icons.Default.Settings
            )
            tabs.forEach { (tab, icon) ->
                val isSelected = selectedTab == tab
                IconButton(onClick = { onTabSelected(tab) }) {
                    Icon(
                        imageVector = icon,
                        contentDescription = null,
                        tint = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(if (isSelected) 28.dp else 24.dp)
                    )
                }
            }
        }
    }
}

@Composable
private fun TerminalExtraKeysBar(
    ctrlActive: Boolean,
    altActive: Boolean,
    onCtrlToggle: () -> Unit,
    onAltToggle: () -> Unit,
    onKey: (String) -> Unit
) {
    val scrollState = rememberScrollState()
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .background(Color(0xFF21252B))
            .horizontalScroll(scrollState)
            .padding(horizontal = 4.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(3.dp)
    ) {
        ExtraKeyButton("ESC", onKey)
        ExtraKeyToggle("CTL", ctrlActive, onCtrlToggle)
        ExtraKeyToggle("ALT", altActive, onAltToggle)
        ExtraKeyButton("TAB", onKey)
        ExtraKeyButton("/", onKey)
        ExtraKeyButton("-", onKey)
        ExtraKeyButton("|", onKey)
        ExtraKeyButton("~", onKey)
        ExtraKeyButton("◀", onClick = { onKey("LEFT") })
        ExtraKeyButton("▲", onClick = { onKey("UP") })
        ExtraKeyButton("▼", onClick = { onKey("DOWN") })
        ExtraKeyButton("▶", onClick = { onKey("RIGHT") })
        ExtraKeyButton("HM", onClick = { onKey("HOME") })
        ExtraKeyButton("EN", onClick = { onKey("END") })
        ExtraKeyButton("PU", onClick = { onKey("PGUP") })
        ExtraKeyButton("PD", onClick = { onKey("PGDN") })
        ExtraKeyButton("DEL", onKey)
    }
}

@Composable
private fun ExtraKeyButton(
    label: String,
    onKey: (String) -> Unit = {},
    onClick: (() -> Unit)? = null
) {
    Surface(
        onClick = { onClick?.invoke() ?: onKey(label) },
        shape = RoundedCornerShape(4.dp),
        color = Color(0xFF3E4451),
        modifier = Modifier.height(30.dp)
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(horizontal = 8.dp)
        ) {
            Text(
                text = label,
                color = Color(0xFFABB2BF),
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Medium
            )
        }
    }
}

@Composable
private fun ExtraKeyToggle(
    label: String,
    active: Boolean,
    onClick: () -> Unit
) {
    Surface(
        onClick = onClick,
        shape = RoundedCornerShape(4.dp),
        color = if (active) Color(0xFF61AFEF) else Color(0xFF3E4451),
        modifier = Modifier.height(30.dp)
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(horizontal = 8.dp)
        ) {
            Text(
                text = label,
                color = if (active) Color(0xFF282C34) else Color(0xFFABB2BF),
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Bold
            )
        }
    }
}
