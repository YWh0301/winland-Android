package com.winland.server

object DistroCatalog {
    val supportedDistros = listOf(
        LinuxDistro(
            id = "ubuntu",
            name = "Ubuntu 24.04 (Minimal)",
            description = "Minimal Ubuntu 24.04 Rootfs",
            url = "https://cdimage.ubuntu.com/ubuntu-base/noble/daily/current/noble-base-arm64.tar.gz"
        ),
        LinuxDistro(
            id = "kali",
            name = "Kali Nethunter (Minimal)",
            description = "Security and Penetration Testing",
            url = "https://kali.download/nethunter-images/current/rootfs/kali-nethunter-rootfs-minimal-arm64.tar.xz"
        )
    )
}