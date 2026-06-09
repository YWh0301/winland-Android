package com.winland.server.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Download
import androidx.compose.material.icons.filled.FormatSize
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Terminal
import androidx.compose.material.icons.filled.Usb
import androidx.compose.material.icons.filled.DisplaySettings
import androidx.compose.material.icons.filled.DarkMode
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material.icons.filled.TouchApp
import androidx.compose.material.icons.filled.Laptop
import androidx.compose.material.icons.filled.Mouse
import androidx.compose.material.icons.filled.LightMode
import androidx.compose.material.icons.filled.PhoneAndroid
import androidx.compose.material.icons.filled.BugReport
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CenterAlignedTopAppBar

import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.LocalTextStyle
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
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
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
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
import com.winland.server.utils.getInstalledDistros

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

    val embeddedTerminal = remember { EmbeddedTerminal(appContext) }
    var ctrlActive by remember { mutableStateOf(false) }
    var altActive by remember { mutableStateOf(false) }
    val terminalDistroId = activeDistroId ?: "ubuntu"
    val keyboardController = LocalSoftwareKeyboardController.current

    LaunchedEffect(selectedTab) {
        keyboardController?.hide()
    }

    LaunchedEffect(terminalDistroId) {
        if (selectedTab == DashboardTab.Terminal) {
            embeddedTerminal.startSession(terminalDistroId)
        }
    }

    Scaffold(
        topBar = {
            if (selectedTab != DashboardTab.Terminal) {
                ProfessionalTopBar(activeOperationText = activeOperationText)
            }
        },
        contentWindowInsets = WindowInsets.systemBars.only(WindowInsetsSides.Top),
    ) { innerPadding ->
        BoxWithConstraints(modifier = Modifier.fillMaxSize().padding(innerPadding)) {
            val isWide = maxWidth >= 1000.dp

            Box(modifier = Modifier.fillMaxSize()) {
                if (selectedTab == DashboardTab.Terminal) {
                    val imeVisible = WindowInsets.ime.getBottom(LocalDensity.current) > 0

                    LaunchedEffect(Unit) {
                        embeddedTerminal.onBarStateChanged = { c, a ->
                            ctrlActive = c
                            altActive = a
                        }
                    }

                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(Color(0xFF282C34))
                            .imePadding()
                            .padding(bottom = if (imeVisible) 0.dp else 84.dp)
                    ) {
                        Box(modifier = Modifier.weight(1f)) {
                            AndroidView(
                                factory = { _ ->
                                    val view = embeddedTerminal.createView()
                                    embeddedTerminal.startSession(terminalDistroId)
                                    view
                                },
                                update = {},
                                modifier = Modifier.fillMaxSize()
                            )
                        }
                        if (imeVisible) {
                            TerminalExtraKeysBar(
                                ctrlActive = ctrlActive,
                                altActive = altActive,
                                onCtrlToggle = {
                                    ctrlActive = !ctrlActive
                                    embeddedTerminal.barCtrlActive = ctrlActive
                                },
                                onAltToggle = {
                                    altActive = !altActive
                                    embeddedTerminal.barAltActive = altActive
                                },
                                onKey = { key ->
                                    embeddedTerminal.sendSpecialKey(key)
                                },
                                modifier = Modifier.fillMaxWidth()
                            )
                        }
                    }
                } else {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(horizontal = 14.dp)
                            .padding(bottom = 84.dp)
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
                                    modifier = Modifier.fillMaxWidth().weight(1f),
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
                                    modifier = Modifier.fillMaxWidth().weight(1f),
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
                                            modifier = Modifier.fillMaxWidth().height(280.dp)
                                        )
                                    }
                                }
                            }
                        } else {
                            SettingsPanel(
                                distros = distros,
                                viewModel = viewModel,
                                followSystemTheme = themeSettings.followSystemTheme,
                                darkModeEnabled = themeSettings.darkModeEnabled,
                                screenPreset = themeSettings.screenPreset,
                                onThemeModeChanged = { followSystem, darkEnabled ->
                                    viewModel.updateThemeMode(followSystem, darkEnabled)
                                },
                                onResolutionApplied = { resolution ->
                                    actions.onShowMessage("Resolution updated to $resolution", false)
                                },
                                onRequestUsb = actions.onRequestUsb,
                                onStopChroot = actions.onDistroStop,
                                onRestartChroot = actions.onDistroRestart,
                                activeDistroId = activeDistroId,
                                controlsEnabled = activeUiOperation == null,
                                modifier = Modifier.fillMaxWidth().weight(1f)
                            )
                        }
                    }
                }

                ModernNavigationBar(
                    selectedTab = selectedTab,
                    onTabSelected = {
                        haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
                        viewModel.setSelectedTab(it)
                    },
                    modifier = Modifier.align(Alignment.BottomCenter)
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
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

            val listState = rememberLazyListState()
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(top = 8.dp)
                    .clip(RoundedCornerShape(14.dp))
                    .background(MaterialTheme.colorScheme.surface)
                    .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(14.dp))
            ) {
                if (displayedLogs.isEmpty()) {
                    Box(
                        modifier = Modifier.fillMaxSize().padding(10.dp),
                        contentAlignment = Alignment.TopStart
                    ) {
                        Text(
                            text = "No logs yet",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            fontFamily = FontFamily.Monospace,
                            fontSize = 12.sp
                        )
                    }
                } else {
                    SelectionContainer {
                        LazyColumn(
                            state = listState,
                            modifier = Modifier.fillMaxSize().padding(horizontal = 10.dp, vertical = 6.dp),
                            verticalArrangement = Arrangement.spacedBy(0.dp)
                        ) {
                            items(
                                count = displayedLogs.size,
                                key = { index -> index }
                            ) { index ->
                                val line = displayedLogs[index]
                                Text(
                                    text = line,
                                    color = MaterialTheme.colorScheme.onSurface,
                                    fontFamily = FontFamily.Monospace,
                                    fontSize = 12.sp,
                                    lineHeight = 16.sp,
                                    modifier = Modifier.animateItemPlacement()
                                )
                            }
                        }
                    }
                    LaunchedEffect(displayedLogs.size) {
                        if (displayedLogs.isNotEmpty()) {
                            listState.animateScrollToItem(displayedLogs.size - 1)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun SettingsPanel(
    distros: List<LinuxDistro>,
    viewModel: MainViewModel,
    followSystemTheme: Boolean,
    darkModeEnabled: Boolean,
    screenPreset: String,
    onThemeModeChanged: (Boolean, Boolean) -> Unit,
    onResolutionApplied: (String) -> Unit,
    onRequestUsb: () -> Unit,
    onStopChroot: () -> Unit,
    onRestartChroot: () -> Unit,
    activeDistroId: String?,
    controlsEnabled: Boolean,
    modifier: Modifier = Modifier
) {
    val appContext = LocalContext.current
    val displayInfo by viewModel.displayInfo.collectAsState()
    var localFollowSystem by remember { mutableStateOf(followSystemTheme) }
    var localDarkMode by remember { mutableStateOf(darkModeEnabled) }
    var localScreenPreset by remember { mutableStateOf(screenPreset) }
    val installedDistros = remember { appContext.getInstalledDistros() }

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
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.Home, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Default Distro", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }
                HorizontalDivider(modifier = Modifier.padding(vertical = 2.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))
                distros.forEach { distro ->
                    val isInstalled = distro.id in installedDistros
                    val isActive = distro.id == activeDistroId
                    val canSelect = isInstalled
                    Surface(
                        onClick = { if (canSelect) viewModel.setActiveDistro(distro.id) },
                        shape = RoundedCornerShape(12.dp),
                        color = if (isActive) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f) else MaterialTheme.colorScheme.surface,
                        tonalElevation = if (isActive) 2.dp else 0.dp,
                        modifier = Modifier.fillMaxWidth(),
                        enabled = canSelect
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 14.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            RadioButton(
                                selected = isActive,
                                onClick = { if (canSelect) viewModel.setActiveDistro(distro.id) },
                                enabled = canSelect
                            )
                            Spacer(Modifier.width(12.dp))
                            Column(modifier = Modifier.weight(1f)) {
                                Text(
                                    text = distro.name,
                                    style = MaterialTheme.typography.bodyLarge,
                                    fontWeight = if (isActive) FontWeight.SemiBold else FontWeight.Normal
                                )
                                Text(
                                    text = if (isInstalled) "Installed" else "Not installed",
                                    style = MaterialTheme.typography.bodySmall,
                                    color = if (isInstalled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error
                                )
                            }
                        }
                    }
                }
            }
        }

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.DisplaySettings, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Display", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }

                Spacer(Modifier.height(4.dp))

                listOf(resolution1080p, resolution720p).forEach { option ->
                    val isSelected = selectedResolution.label == option.label
                    Surface(
                        onClick = {
                            selectedResolutionLabel = option.label
                            NativeBridge.setScaleSafe(option.scale)
                            onResolutionApplied("${option.label}: scale=${option.scale}")
                        },
                        shape = RoundedCornerShape(12.dp),
                        color = if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f) else MaterialTheme.colorScheme.surface,
                        tonalElevation = if (isSelected) 2.dp else 0.dp,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 14.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            RadioButton(
                                selected = isSelected,
                                onClick = {
                                    selectedResolutionLabel = option.label
                                    NativeBridge.setScaleSafe(option.scale)
                                    onResolutionApplied("${option.label}: scale=${option.scale}")
                                }
                            )
                            Spacer(Modifier.width(12.dp))
                            Text(
                                text = option.label,
                                style = MaterialTheme.typography.bodyLarge,
                                fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal
                            )
                        }
                    }
                }
            }
        }


        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.DisplaySettings, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Display Info", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }

                HorizontalDivider(modifier = Modifier.padding(vertical = 4.dp), color = MaterialTheme.colorScheme.outlineVariant.copy(alpha = 0.5f))

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
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.DarkMode, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Appearance", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }

                Text(
                    text = "Theme",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    data class ThemeOption(val label: String, val icon: androidx.compose.ui.graphics.vector.ImageVector, val onClick: () -> Unit, val isSelected: () -> Boolean)
                    val themeOptions = listOf(
                        ThemeOption("System", Icons.Default.PhoneAndroid, { localFollowSystem = true; onThemeModeChanged(true, localDarkMode) }, { localFollowSystem }),
                        ThemeOption("Dark", Icons.Default.DarkMode, { localFollowSystem = false; localDarkMode = true; onThemeModeChanged(false, true) }, { !localFollowSystem && localDarkMode }),
                        ThemeOption("Light", Icons.Default.LightMode, { localFollowSystem = false; localDarkMode = false; onThemeModeChanged(false, false) }, { !localFollowSystem && !localDarkMode }),
                    )
                    themeOptions.forEach { option ->
                        val isSelected = option.isSelected()
                        Surface(
                            onClick = option.onClick,
                            shape = RoundedCornerShape(12.dp),
                            color = if (isSelected) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.5f),
                            modifier = Modifier.weight(1f)
                        ) {
                            Column(
                                modifier = Modifier.padding(vertical = 12.dp),
                                horizontalAlignment = Alignment.CenterHorizontally
                            ) {
                                Icon(
                                    imageVector = option.icon,
                                    contentDescription = option.label,
                                    tint = if (isSelected) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.size(20.dp)
                                )
                                Spacer(Modifier.height(4.dp))
                                Text(
                                    text = option.label,
                                    style = MaterialTheme.typography.labelLarge,
                                    fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal
                                )
                            }
                        }
                    }
                }
            }
        }

        ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh)) {
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.PowerSettingsNew, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Runtime Controls", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }
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
            Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(Icons.Default.TouchApp, contentDescription = null, tint = MaterialTheme.colorScheme.primary, modifier = Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Input Mode", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
                }

                Spacer(Modifier.height(4.dp))

                val currentMode by viewModel.inputMode.collectAsState()

                MainViewModel.InputMode.entries.forEach { mode ->
                    val isSelected = currentMode == mode
                    Surface(
                        onClick = { viewModel.updateInputMode(mode) },
                        shape = RoundedCornerShape(12.dp),
                        color = if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f) else MaterialTheme.colorScheme.surface,
                        tonalElevation = if (isSelected) 2.dp else 0.dp,
                        modifier = Modifier.fillMaxWidth()
                    ) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(horizontal = 16.dp, vertical = 14.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            RadioButton(
                                selected = isSelected,
                                onClick = { viewModel.updateInputMode(mode) }
                            )
                            Spacer(Modifier.width(12.dp))
                            Text(
                                text = "${mode.name} Mode",
                                style = MaterialTheme.typography.bodyLarge,
                                fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Medium
                            )
                            Spacer(Modifier.weight(1f))
                            Icon(
                                imageVector = when (mode) {
                                    MainViewModel.InputMode.Touch -> Icons.Default.TouchApp
                                    MainViewModel.InputMode.Trackpad -> Icons.Default.Laptop
                                    MainViewModel.InputMode.Mouse -> Icons.Default.Mouse
                                },
                                contentDescription = null,
                                tint = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.size(20.dp)
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
    val context = LocalContext.current
    val haptics = LocalHapticFeedback.current
    val activeUiOperation by viewModel.activeUiOperation.collectAsState()
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
    val perDistroStates by viewModel.perDistroChrootState.collectAsState()
    val refreshSignal by viewModel.perDistroRefreshSignal.collectAsState()
    val distroChrootState = perDistroStates[distro.id]

    LaunchedEffect(distro.id, refreshSignal) {
        viewModel.refreshChrootRuntimeStateForDistro(distro.id)
    }

    val currentStage = when {
        distroChrootState?.ready == true -> "run"
        distroChrootState?.isExtracted == true -> "setup"
        else -> "install"
    }
    val statusReady = distroChrootState?.ready == true
    val statusText = distroChrootState?.reason ?: "Checking status..."

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
                    Text(distro.description, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
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
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 5.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .clip(CircleShape)
                    .background(if (ready) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f))
            )
            Spacer(Modifier.width(6.dp))
            Text(
                text = if (ready) "READY" else "PENDING",
                style = MaterialTheme.typography.labelSmall,
                color = if (ready) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
                fontWeight = FontWeight.Bold
            )
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProfessionalTopBar(activeOperationText: String?) {
    CenterAlignedTopAppBar(
        title = {
            Column(horizontalAlignment = Alignment.CenterHorizontally) {
                Text("Winland", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
                if (activeOperationText != null) {
                    Text(
                        activeOperationText,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary
                    )
                } else {
                    Text(
                        "Linux Desktop Environment",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant
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
private fun ModernNavigationBar(selectedTab: DashboardTab, onTabSelected: (DashboardTab) -> Unit, modifier: Modifier = Modifier) {
    Surface(
        modifier = modifier
            .padding(horizontal = 24.dp, vertical = 12.dp)
            .height(72.dp)
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
                Triple(DashboardTab.Home, Icons.Default.Home, "Home"),
                Triple(DashboardTab.Terminal, Icons.Default.Terminal, "Terminal"),
                Triple(DashboardTab.Settings, Icons.Default.Settings, "Settings")
            )
            tabs.forEach { (tab, icon, label) ->
                val isSelected = selectedTab == tab
                val color by animateColorAsState(
                    targetValue = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                    label = "tabColor"
                )
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    modifier = Modifier
                        .clip(RoundedCornerShape(16.dp))
                        .then(
                            Modifier
                                .background(
                                    if (isSelected) MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.3f)
                                    else MaterialTheme.colorScheme.surface.copy(alpha = 0f)
                                )
                                .padding(horizontal = 16.dp, vertical = 6.dp)
                        )
                ) {
                    IconButton(
                        onClick = { onTabSelected(tab) },
                        modifier = Modifier.size(28.dp)
                    ) {
                        Icon(
                            imageVector = icon,
                            contentDescription = label,
                            tint = color,
                            modifier = Modifier.size(if (isSelected) 24.dp else 22.dp)
                        )
                    }
                    Text(
                        text = label,
                        style = MaterialTheme.typography.labelSmall,
                        color = color,
                        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Normal,
                        fontSize = 10.sp
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
    onKey: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    val scrollState = rememberScrollState()
    Row(
        modifier = modifier
            .fillMaxWidth()
            .background(Color(0xFF1A1D23))
            .horizontalScroll(scrollState)
            .padding(horizontal = 6.dp, vertical = 5.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
        verticalAlignment = Alignment.CenterVertically
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
    val haptics = LocalHapticFeedback.current
    Surface(
        onClick = {
            haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            onClick?.invoke() ?: onKey(label)
        },
        shape = RoundedCornerShape(8.dp),
        color = Color(0xFF3E4451),
        modifier = Modifier.height(40.dp)
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(horizontal = 12.dp)
        ) {
            Text(
                text = label,
                color = Color(0xFFABB2BF),
                fontSize = 13.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.SemiBold
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
    val haptics = LocalHapticFeedback.current
    val bgColor by animateColorAsState(
        targetValue = if (active) Color(0xFF61AFEF) else Color(0xFF3E4451),
        label = "toggleBg"
    )
    val textColor = if (active) Color(0xFF282C34) else Color(0xFFABB2BF)
    Surface(
        onClick = {
            haptics.performHapticFeedback(HapticFeedbackType.TextHandleMove)
            onClick()
        },
        shape = RoundedCornerShape(8.dp),
        color = bgColor,
        modifier = Modifier.height(40.dp)
    ) {
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier.padding(horizontal = 12.dp)
        ) {
            Text(
                text = label,
                color = textColor,
                fontSize = 13.sp,
                fontFamily = FontFamily.Monospace,
                fontWeight = FontWeight.Bold
            )
        }
    }
}
