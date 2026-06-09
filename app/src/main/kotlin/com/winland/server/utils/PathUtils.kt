package com.winland.server.utils

import android.content.Context

/**
 * Utility to provide a unified path for application files.
 * Forces the use of /data/data/ instead of /data/user/0/ to maintain 
 * consistency across chroot and internal tooling.
 */
fun Context.getUnifiedFilesDir(): String {
    return filesDir.absolutePath.replace("/data/user/0/", "/data/data/")
}

fun Context.getUnifiedTmpDir(): String {
    return "${getUnifiedFilesDir()}/tmp"
}

fun Context.getUnifiedRootfsDir(distroId: String = "ubuntu"): String {
    return "${getUnifiedFilesDir()}/rootfs_$distroId"
}

fun Context.getInstalledDistros(): List<String> {
    val filesDir = getUnifiedFilesDir()
    val dir = java.io.File(filesDir)
    return dir.listFiles()
        ?.filter { it.isDirectory && it.name.startsWith("rootfs_") }
        ?.map { it.name.removePrefix("rootfs_") }
        ?: emptyList()
}
