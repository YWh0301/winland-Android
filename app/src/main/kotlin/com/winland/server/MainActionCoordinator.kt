package com.winland.server

import android.content.Context
import android.util.Log
import com.winland.server.engine.usecase.DownloadAndExtractRootfsUseCase
import com.winland.server.engine.usecase.PrepareRunUseCase
import com.winland.server.engine.usecase.RestartChrootUseCase
import com.winland.server.engine.usecase.SetupRootfsUseCase
import com.winland.server.engine.usecase.StopChrootUseCase
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

fun interface RuntimeLauncher {
    fun launchRuntime()
}

class MainActionCoordinator(
    private val appContext: Context,
    private val scope: CoroutineScope,
    private val viewModel: MainViewModel,
    private val showToast: (String, Boolean) -> Unit,
    private val runtimeLauncher: RuntimeLauncher
) {
    private val downloadAndExtractRootfsUseCase = DownloadAndExtractRootfsUseCase()
    private val setupRootfsUseCase = SetupRootfsUseCase()
    private val prepareRunUseCase = PrepareRunUseCase()
    private val stopChrootUseCase = StopChrootUseCase()
    private val restartChrootUseCase = RestartChrootUseCase()

    fun handleDistroInstall(distro: LinuxDistro) {
        if (!viewModel.tryBeginOperation(MainViewModel.UiOperation.DOWNLOAD)) {
            showToast("Another operation is running. Wait until it finishes.", false)
            return
        }

        viewModel.setDistroDownloading(distro.id, true)
        viewModel.setDistroProgress(distro.id, 0f)
        scope.launch(Dispatchers.IO) {
            Log.d("WinlandDownloader", "Download started for ${distro.name}...")
            val result = try {
                downloadAndExtractRootfsUseCase(
                    context = appContext,
                    distro = distro,
                    onProgress = { progress -> viewModel.setDistroProgress(distro.id, progress) }
                )
            } finally {
                viewModel.setDistroDownloading(distro.id, false)
            }

            withContext(Dispatchers.Main) {
                if (result.success) {
                    viewModel.setActiveDistro(distro.id)
                    viewModel.refreshChrootRuntimeState()
                    viewModel.updateDistroStage(distro.id, "setup")
                    showToast("Download + extract complete. Press Setup.", false)
                } else {
                    viewModel.updateDistroStage(distro.id, "Download/Extract failed; retry install")
                    if (result.failurePhase == DownloadAndExtractRootfsUseCase.FailurePhase.EXTRACT) {
                        showToast("Extract failed. Re-download rootfs.", true)
                    } else {
                        showToast("Download failed", true)
                    }
                }
                viewModel.endOperation(MainViewModel.UiOperation.DOWNLOAD)
            }
        }
    }

    fun handleDistroSetup(distroId: String) {
        if (!viewModel.tryBeginOperation(MainViewModel.UiOperation.SETUP)) {
            showToast("Another operation is running. Wait until it finishes.", false)
            return
        }

        scope.launch(Dispatchers.IO) {
            try {
                val setupResult = setupRootfsUseCase(appContext, distroId)
                withContext(Dispatchers.Main) {
                    viewModel.refreshChrootRuntimeState()
                    viewModel.updateDistroStage(distroId, setupResult.stageText)
                    showToast(setupResult.message, !setupResult.success)
                }
            } finally {
                viewModel.endOperation(MainViewModel.UiOperation.SETUP)
            }
        }
    }

    fun handleDistroRun(distroId: String) {
        if (!viewModel.tryBeginOperation(MainViewModel.UiOperation.RUN)) {
            showToast("Another operation is running. Wait until it finishes.", false)
            return
        }

        viewModel.setDistroRunLaunching(distroId, true)
        viewModel.updateDistroStage(distroId, "Run launching")
        scope.launch(Dispatchers.IO) {
            try {
                val runPreparation = prepareRunUseCase(appContext, distroId)
                if (!runPreparation.canLaunch) {
                    withContext(Dispatchers.Main) {
                        showToast(runPreparation.message, true)
                        viewModel.updateDistroStage(distroId, "Run blocked")
                    }
                    return@launch
                }

                withContext(Dispatchers.Main) {
                    runtimeLauncher.launchRuntime()
                    viewModel.refreshChrootRuntimeState()
                    viewModel.updateDistroStage(distroId, runPreparation.message)
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    showToast("Run failed: ${e.message}", true)
                    viewModel.updateDistroStage(distroId, "Run failed")
                }
            } finally {
                withContext(Dispatchers.Main) {
                    viewModel.setDistroRunLaunching(distroId, false)
                }
                viewModel.endOperation(MainViewModel.UiOperation.RUN)
            }
        }
    }

    fun handleDistroStop() {
        val distroId = viewModel.activeDistroId.value
        if (distroId == null) {
            showToast("No active distro to stop", false)
            return
        }
        scope.launch(Dispatchers.IO) {
            val result = stopChrootUseCase(appContext, distroId)
            withContext(Dispatchers.Main) {
                if (result.isSuccess) {
                    viewModel.refreshChrootRuntimeState()
                    showToast("Chroot stopped", false)
                } else {
                    showToast("Stop failed: ${result.exceptionOrNull()?.message}", true)
                }
            }
        }
    }

    fun handleDistroRestart() {
        val distroId = viewModel.activeDistroId.value
        if (distroId == null) {
            showToast("No active distro to restart", false)
            return
        }
        scope.launch(Dispatchers.IO) {
            val result = restartChrootUseCase(appContext, distroId)
            withContext(Dispatchers.Main) {
                if (result.isSuccess) {
                    viewModel.refreshChrootRuntimeState()
                    showToast("Chroot restarted", false)
                } else {
                    showToast("Restart failed: ${result.exceptionOrNull()?.message}", true)
                }
            }
        }
    }
}