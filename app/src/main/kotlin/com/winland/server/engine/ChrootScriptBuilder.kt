package com.winland.server.engine

object ChrootScriptBuilder {
    private const val XDG_RUNTIME_DIR_VAL = "/run/user/0"
    private const val PULSE_SOCKET_PATH = "/tmp/pulse-runtime/native"
    private const val PULSE_SERVER_VAL = "unix:/tmp/pulse-runtime/native"

    fun buildExtractScript(
        filesDir: String,
        downloadsDir: String,
        rootfsDir: String,
        stagedRootfsDir: String,
        backupRootfsDir: String,
        tmpDir: String,
        profileInstalledDir: String,
        filename: String,
        extractedMarker: String,
        installedMarker: String
    ): String {
        return """
            #!/system/bin/sh
            set -e

            INTERNAL_BB="$filesDir/bin/busybox"
            if [ -x "${'$'}INTERNAL_BB" ]; then
                BB="${'$'}INTERNAL_BB"
            else
                BB=$(command -v busybox || echo "busybox")
            fi

            ensure_directory() {
                target="$1"
                if [ -e "${'$'}target" ] && [ ! -d "${'$'}target" ]; then
                    rm -f "${'$'}target"
                fi
                mkdir -p "${'$'}target"
            }

            unmount_safe() {
                target="$1"
                "${'$'}BB" umount -l "${'$'}target" 2>/dev/null || true
            }

            unmount_all_under_rootfs() {
                BASE_DIR="$rootfsDir"
                cat /proc/mounts | grep "${'$'}BASE_DIR" | awk '{print ${'$'}2}' | sort -r | while read -r mnt; do
                    "${'$'}BB" umount -l "${'$'}mnt" || true
                done
            }

            echo "PREPARING DIRECTORY LAYOUT..."
            ensure_directory $downloadsDir

            pkill -f startxfce4 2>/dev/null || true
            pkill -f xfce4-session 2>/dev/null || true
            pkill -f weston 2>/dev/null || true
            pkill -f Xwayland 2>/dev/null || true
            pkill -f pulseaudio 2>/dev/null || true
            pkill -f dbus-daemon 2>/dev/null || true

            unmount_all_under_rootfs
            unmount_safe $rootfsDir/dev/shm
            unmount_safe $rootfsDir/external_storage
            unmount_safe $rootfsDir/tmp
            unmount_safe $rootfsDir/dev/pts
            unmount_safe $rootfsDir/dev
            unmount_safe $rootfsDir/proc
            unmount_safe $rootfsDir/sys
            unmount_all_under_rootfs

            rm -rf $stagedRootfsDir
            ensure_directory $stagedRootfsDir
            ensure_directory $tmpDir
            ensure_directory $profileInstalledDir
            rm -f $profileInstalledDir/$extractedMarker $profileInstalledDir/$installedMarker || true

            echo "EXTRACTING ROOTFS WITH TAR..."
            ARCHIVE_PATH="$downloadsDir/$filename"
            extract_ok=0

            case "${'$'}filename" in
                *.xz)
                    echo "INFO: detected XZ archive, using -xJf..."
                    if "${'$'}BB" tar -xJf "${'$'}ARCHIVE_PATH" -C "$stagedRootfsDir"; then
                        extract_ok=1
                    fi
                    ;;
                *.gz|*.tgz)
                    echo "INFO: detected GZIP archive, using -xzf..."
                    if "${'$'}BB" tar -xzf "${'$'}ARCHIVE_PATH" -C "$stagedRootfsDir"; then
                        extract_ok=1
                    fi
                    ;;
                *)
                    echo "INFO: unknown extension, trying auto-detection..."
                    if "${'$'}BB" tar -xf "${'$'}ARCHIVE_PATH" -C "$stagedRootfsDir"; then
                        extract_ok=1
                    fi
                    ;;
            esac

            if [ "${'$'}extract_ok" -ne 1 ]; then
                echo "WARN: extension-based extraction failed, trying broad fallbacks..."
                rm -rf "$stagedRootfsDir"/*
                if "${'$'}BB" tar -xf "${'$'}ARCHIVE_PATH" -C "$stagedRootfsDir"; then
                    extract_ok=1
                fi
            fi

            if [ "${'$'}extract_ok" -ne 1 ]; then
                echo "ERROR: rootfs extraction failed; archive may be incomplete or invalid"
                rm -rf "$stagedRootfsDir" || true
                rm -f "${'$'}ARCHIVE_PATH" || true
                echo "ERROR: removed invalid archive; please download again"
                exit 1
            fi

            echo "CHECKING FOR WRAPPING DIRECTORY..."
            if [ -d "$stagedRootfsDir" ]; then
                dir_contents=$("${'$'}BB" ls -A "$stagedRootfsDir")
                first_entry=$(echo "${'$'}dir_contents" | head -1)
                entry_count=$(echo "${'$'}dir_contents" | "${'$'}BB" wc -l)
                if [ "${'$'}entry_count" -eq 1 ] && [ -d "$stagedRootfsDir/${'$'}first_entry" ] && [ "${'$'}first_entry" != "." ] && [ "${'$'}first_entry" != ".." ]; then
                    echo "INFO: detected wrapping directory '${'$'}first_entry', relocating..."
                    tmp_move="${stagedRootfsDir}_inner"
                    mv "$stagedRootfsDir/${'$'}first_entry" "${'$'}tmp_move"
                    rm -rf "$stagedRootfsDir"
                    mv "${'$'}tmp_move" "$stagedRootfsDir"
                    echo "INFO: relocation complete"
                fi
            fi

            echo "VALIDATING EXTRACTED ROOTFS..."
            if [ -x "$stagedRootfsDir/bin/bash" ]; then
                :
            elif [ -x "$stagedRootfsDir/bin/dash" ]; then
                echo "INFO: /bin/bash not found, symlinking /bin/dash -> /bin/bash"
                ln -s dash "$stagedRootfsDir/bin/bash"
            elif [ -x "$stagedRootfsDir/bin/sh" ]; then
                echo "INFO: /bin/bash not found, symlinking /bin/sh -> /bin/bash"
                ln -s sh "$stagedRootfsDir/bin/bash"
            else
                echo "ERROR: rootfs invalid (missing /bin/bash, /bin/dash, or /bin/sh)"
                exit 1
            fi
            [ -x "$stagedRootfsDir/usr/bin/grep" ] || { echo "ERROR: rootfs invalid (missing /usr/bin/grep)"; exit 1; }
            [ -x "$stagedRootfsDir/usr/bin/apt-get" ] || { echo "ERROR: rootfs invalid (missing /usr/bin/apt-get)"; exit 1; }

            echo "FINALIZING ROOTFS SWAP..."
            rm -rf "$backupRootfsDir" || true
            if [ -d "$rootfsDir" ]; then
                mv "$rootfsDir" "$backupRootfsDir" || true
            fi
            mv "$stagedRootfsDir" "$rootfsDir"

            touch $profileInstalledDir/$extractedMarker
            echo "Extract Done."
        """.trimIndent()
    }

    fun buildSetupScript(
        filesDir: String,
        rootfsDir: String,
        tmpDir: String,
        profileInstalledDir: String,
        externalStoragePath: String,
        installedMarker: String
    ): String {
        return """
            #!/system/bin/sh
            set -e
            set -x

            INTERNAL_BB="$filesDir/bin/busybox"
            if [ -x "${'$'}INTERNAL_BB" ]; then
                BB="${'$'}INTERNAL_BB"
            else
                BB=$(command -v busybox || echo "busybox")
            fi

            LOG_FILE="$tmpDir/setup.log"
            echo "SETUP START: $(date)" > "${'$'}LOG_FILE"
            exec 2>>"${'$'}LOG_FILE"

            ensure_directory() {
                target="$1"
                if [ -e "${'$'}target" ] && [ ! -d "${'$'}target" ]; then
                    rm -f "${'$'}target"
                fi
                mkdir -p "${'$'}target"
            }

            mount_safe() {
                if ! grep -q " ${'$'}2 " /proc/mounts; then
                    "${'$'}BB" mount -t "${'$'}3" "${'$'}2" "${'$'}2" 2>/dev/null || "${'$'}BB" mount -o bind "${'$'}2" "${'$'}2" 2>/dev/null || echo "Warning: mount ${'$'}3 on ${'$'}2 failed"
                fi
            }

            echo "SETUP: preparing mounts"
            "${'$'}BB" mount -o remount,dev,suid /data 2>/dev/null || true

            ensure_directory $profileInstalledDir
            ensure_directory $tmpDir
            chmod 1777 $tmpDir

            mkdir -p $rootfsDir/proc $rootfsDir/sys $rootfsDir/dev $rootfsDir/dev/pts $rootfsDir/external_storage

            "${'$'}BB" mount -o bind $tmpDir $rootfsDir/tmp || echo "Warning: failed to bind mount /tmp" >> "${'$'}LOG_FILE"

            echo "$(date): Starting critical mounts..." >> "${'$'}LOG_FILE"
            "${'$'}BB" mount -t proc proc $rootfsDir/proc || echo "proc mount failed" >> "${'$'}LOG_FILE"
            "${'$'}BB" mount -t sysfs sys $rootfsDir/sys || echo "sys mount failed" >> "${'$'}LOG_FILE"
            "${'$'}BB" mount -o bind /dev $rootfsDir/dev || echo "dev mount failed" >> "${'$'}LOG_FILE"
            "${'$'}BB" mount -o bind /dev/pts $rootfsDir/dev/pts || echo "pts mount failed" >> "${'$'}LOG_FILE"
            "${'$'}BB" mount -o bind $externalStoragePath $rootfsDir/external_storage || true

            mkdir -p $rootfsDir/dev/shm
            "${'$'}BB" mount -t tmpfs -o nosuid,nodev tmpfs $rootfsDir/dev/shm || echo "shm mount failed" >> "${'$'}LOG_FILE"
            chmod 1777 $rootfsDir/dev/shm || true

            echo "SETUP: fixing GPU permissions"
            chmod 666 $rootfsDir/dev/dri/* 2>/dev/null || true
            chmod 666 $rootfsDir/dev/kgsl-3d0 2>/dev/null || true
            chmod 666 $rootfsDir/dev/ion 2>/dev/null || true

            echo "SETUP: fixing apt and dns"
            mkdir -p $rootfsDir/etc/apt/apt.conf.d
            cat > $rootfsDir/etc/apt/apt.conf.d/01-android-nosandbox << 'EOF'
APT::Sandbox::User "root";
EOF
            cat > $rootfsDir/etc/resolv.conf << 'EOF'
nameserver 1.1.1.1
nameserver 8.8.8.8
options timeout:2 attempts:3
EOF

            chroot $rootfsDir /bin/bash -c '
                export HOME=/root
                export USER=root
                export LOGNAME=root
                export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
                /usr/bin/grep -q "^aid_inet:" /etc/group || /usr/sbin/groupadd -g 3003 aid_inet || true
                /usr/bin/id _apt >/dev/null 2>&1 && /usr/sbin/usermod -G nogroup -g aid_inet _apt || true

                echo "SETUP: spoofing battery for upower/xfce"
                mkdir -p /etc/UPower
                cat <<UP_EOF > /etc/UPower/UPower.conf
[UPower]
EnableEloy=false
UsePercentageForPolicy=true
ShortTermHistory=true
LongTermHistory=true
UP_EOF
                mkdir -p /var/lib/upower
            '

            echo "SETUP: configuring profile"
            touch $rootfsDir/etc/profile
            echo 'export DISPLAY=:1' >> $rootfsDir/etc/profile
            echo 'export XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR_VAL' >> $rootfsDir/etc/profile

            echo "SETUP: executing setup.sh"
            chmod +x $rootfsDir/tmp/setup.sh
            "${'$'}BB" chroot $rootfsDir /bin/bash -c '
                export HOME=/root
                export USER=root
                export LOGNAME=root
                export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
                setenforce 0 >/dev/null 2>&1 || true
                /bin/bash /tmp/setup.sh
            '

            touch $profileInstalledDir/$installedMarker
            echo "SETUP: done"
        """.trimIndent()
    }

    fun buildRunScript(
        filesDir: String,
        rootfsDir: String,
        tmpDir: String,
        externalStoragePath: String,
        density: Float,
        distroId: String
    ): String {
        val otherDistroId = if (distroId == "ubuntu") "kali" else "ubuntu"
        val otherRootfsDir = "$filesDir/rootfs_$otherDistroId"
        val desktopScale = density.coerceIn(0.5f, 1.5f)
        return """
            #!/system/bin/sh
            set +e
            set -x

            INTERNAL_BB="$filesDir/bin/busybox"
            if [ -x "${'$'}INTERNAL_BB" ]; then
                BB="${'$'}INTERNAL_BB"
            else
                BB=$(command -v busybox || echo "busybox")
            fi

            setenforce 0 > /dev/null 2>&1 || true

            RUNTIME_LOG=$tmpDir/chroot-run.log
            echo "-----------------------------------" >> "${'$'}RUNTIME_LOG"
            echo "RUN START: $(date)" >> "${'$'}RUNTIME_LOG"
            exec 2>>"${'$'}RUNTIME_LOG"

            on_exit() {
                rc=${'$'}?
                echo "$(date '+%Y-%m-%d %H:%M:%S') SCRIPT_EXIT rc=${'$'}{rc}" >> "${'$'}RUNTIME_LOG" || true
            }
            trap on_exit EXIT

            log_runtime() {
                echo "$(date '+%Y-%m-%d %H:%M:%S') ${'$'}1" >> "$tmpDir/chroot-run.log"
            }

            ensure_directory() {
                target="${'$'}1"
                if [ -e "${'$'}target" ] && [ ! -d "${'$'}target" ]; then
                    rm -f "${'$'}target"
                fi
                mkdir -p "${'$'}target"
            }

            mount_bind_if_needed() {
                source="${'$'}1"
                target="${'$'}2"
                ensure_directory "${'$'}target"
                if ! grep -q " ${'$'}target " /proc/mounts; then
                    "${'$'}BB" mount -o bind "${'$'}source" "${'$'}target" 2>/dev/null || true
                fi
            }

            mount_fs_if_needed() {
                fs_type="${'$'}1"
                source="${'$'}2"
                target="${'$'}3"
                options="${'$'}4"
                ensure_directory "${'$'}target"
                if ! grep -q " ${'$'}target " /proc/mounts; then
                    if [ -n "${'$'}options" ]; then
                        "${'$'}BB" mount -t "${'$'}fs_type" -o "${'$'}options" "${'$'}source" "${'$'}target" 2>/dev/null || true
                    else
                        "${'$'}BB" mount -t "${'$'}fs_type" "${'$'}source" "${'$'}target" 2>/dev/null || true
                    fi
                fi
            }

            log_runtime "RUN: mutally unmounting other distro"
            OTHER_ROOTFS="$otherRootfsDir"
            if [ -d "${'$'}OTHER_ROOTFS" ]; then
                cat /proc/mounts | grep "${'$'}OTHER_ROOTFS" | awk '{print ${'$'}2}' | sort -r | while read -r mnt; do
                    "${'$'}BB" umount -l "${'$'}mnt" 2>/dev/null || true
                done
            fi

            log_runtime "RUN: preparing runtime mounts"
            "${'$'}BB" mount -o remount,dev,suid /data 2>/dev/null || true
            SHARED_SOCKET_DIR="$filesDir/tmp"
            GUEST_SHARED_SOCKET_DIR="$rootfsDir$filesDir/tmp"
            ensure_directory "$tmpDir"
            ensure_directory "${'$'}SHARED_SOCKET_DIR"

            if [ -S "${'$'}SHARED_SOCKET_DIR/wayland-0" ]; then
                chmod 777 "${'$'}SHARED_SOCKET_DIR/wayland-0" 2>/dev/null || true
            fi

            ensure_directory $rootfsDir/proc
            ensure_directory $rootfsDir/sys
            ensure_directory $rootfsDir/dev
            ensure_directory $rootfsDir/dev/pts
            ensure_directory $rootfsDir/external_storage

            mount_fs_if_needed proc proc "$rootfsDir/proc"
            mount_fs_if_needed sysfs sys "$rootfsDir/sys"
            mount_bind_if_needed /dev "$rootfsDir/dev"
            mount_bind_if_needed /dev/pts "$rootfsDir/dev/pts"
            mount_bind_if_needed $externalStoragePath "$rootfsDir/external_storage"

            ensure_directory $rootfsDir/dev/shm
            mount_fs_if_needed tmpfs tmpfs "$rootfsDir/dev/shm" "nosuid,nodev"
            chmod 1777 $rootfsDir/dev/shm || true
            ensure_directory $rootfsDir/tmp
            mount_bind_if_needed $tmpDir "$rootfsDir/tmp"
            ensure_directory "${'$'}GUEST_SHARED_SOCKET_DIR"
            mount_bind_if_needed $tmpDir "${'$'}GUEST_SHARED_SOCKET_DIR"

            ensure_directory $rootfsDir/tmp/pulse-runtime
            chmod 777 $rootfsDir/tmp/pulse-runtime

            log_runtime "RUN: starting desktop session"
            echo "$(date): Starting chroot environment..." >> "${'$'}RUNTIME_LOG"

            chroot $rootfsDir /usr/bin/env -i \
                HOME=/root \
                TERM=xterm \
                PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                WAYLAND_DISPLAY=wayland-0 \
                XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR_VAL \
                WINLAND_SOCKET_DIR=$filesDir/tmp \
                PULSE_SERVER=$PULSE_SERVER_VAL \
                /bin/bash -l <<'CHROOT_EOF'

                log_runtime_guest() {
                    echo "$(date '+%Y-%m-%d %H:%M:%S') ${'$'}1" >> /tmp/chroot-run.log
                }

                run_in_dbus_session() {
                    if command -v dbus-run-session >/dev/null 2>&1; then
                        dbus-run-session -- "$@"
                    else
                        eval "$@"
                    fi
                }

                export PRIVATE_RUNTIME_DIR=$XDG_RUNTIME_DIR_VAL
                export HOME=/root
                export USER=root
                export LOGNAME=root

                [ ! -s /etc/machine-id ] && dbus-uuidgen > /etc/machine-id 2>/dev/null || true

                mkdir -p "${'$'}PRIVATE_RUNTIME_DIR"
                chmod 700 "${'$'}PRIVATE_RUNTIME_DIR" || true
                export XDG_RUNTIME_DIR="${'$'}PRIVATE_RUNTIME_DIR"

                echo "Starting D-Bus Session Bus..."
                if command -v dbus-launch >/dev/null 2>&1; then
                    eval $(dbus-launch --sh-syntax)
                    log_runtime_guest "D-Bus session bus started: DBUS_SESSION_BUS_ADDRESS=${'$'}DBUS_SESSION_BUS_ADDRESS"
                fi

                echo "Starting PulseAudio..."
                chmod 1777 /tmp 2>/dev/null || true
                mkdir -p /tmp/pulse-runtime
                chmod 777 /tmp/pulse-runtime 2>/dev/null || true
                rm -rf /tmp/pulse-verbose.log $PULSE_SOCKET_PATH /tmp/pulse.pa
                mkdir -p /root/.config/pulse
                cat <<CLIENT_EOF > /root/.config/pulse/client.conf
                autospawn = no
                default-server = $PULSE_SERVER_VAL
CLIENT_EOF
                mkdir -p /tmp/audio_bridge
                chmod 777 /tmp/audio_bridge
                mkfifo -m 666 /tmp/audio_bridge/fifo 2>/dev/null || true
                exec 3<>/tmp/audio_bridge/fifo
                cat <<PULSE_EOF > /tmp/pulse.pa
                load-module module-native-protocol-unix auth-anonymous=1 socket=$PULSE_SOCKET_PATH
                load-module module-pipe-sink sink_name=AndroidSink file=/tmp/audio_bridge/fifo format=s16le channels=2 rate=44100
                set-default-sink AndroidSink
                load-module module-pipe-source source_name=AndroidMic file=/tmp/audio_bridge/mic_fifo format=s16le channels=1 rate=44100
                set-default-source AndroidMic
PULSE_EOF

                chmod 755 /run 2>/dev/null || true
                mkdir -p /var/run/pulse
                chown pulse:pulse /var/run/pulse 2>/dev/null || chown 101:102 /var/run/pulse 2>/dev/null || true
                unset PULSE_SERVER
                rm -f /run/user/0/pulse/pid 2>/dev/null || true
                pulseaudio -n -F /tmp/pulse.pa --daemonize --system --realtime=no --disallow-exit --exit-idle-time=-1 --disable-shm=yes --use-pid-file=false --log-target=file:/tmp/pulse-verbose.log -vvvv || true
                pulse_wait_ok=0
                j=0
                while [ "${'$'}j" -lt 10 ]; do
                    if [ -S $PULSE_SOCKET_PATH ]; then
                        chmod 777 $PULSE_SOCKET_PATH 2>/dev/null || true
                        pulse_wait_ok=1
                        log_runtime_guest "RUN: confirmed pulse socket at $PULSE_SOCKET_PATH"
                        break
                    fi
                    sleep 1
                    j=${'$'}((j + 1))
                done
                export PULSE_SERVER=$PULSE_SERVER_VAL
                if [ "${'$'}pulse_wait_ok" -ne 1 ]; then
                    log_runtime_guest "WARN: pulse socket missing at $PULSE_SOCKET_PATH"
                fi

                export WAYLAND_DISPLAY=wayland-0

                wayland_wait_ok=0
                i=0
                log_runtime_guest "Waiting for ${'$'}WINLAND_SOCKET_DIR/wayland-0 (max 30s)..."
                while [ "${'$'}i" -lt 30 ]; do
                    if [ -S "${'$'}WINLAND_SOCKET_DIR/wayland-0" ]; then
                        wayland_wait_ok=1
                        log_runtime_guest "RUN: confirmed native ${'$'}WINLAND_SOCKET_DIR/wayland-0 socket"
                        break
                    fi
                    sleep 1
                    i=${'$'}((i + 1))
                done

                if [ "${'$'}wayland_wait_ok" -ne 1 ]; then
                    log_runtime_guest "FATAL: ${'$'}WINLAND_SOCKET_DIR/wayland-0 missing. Native bridge failed."
                    exit 1
                fi

                mkdir -p "${'$'}XDG_RUNTIME_DIR"
                ln -sfn "${'$'}WINLAND_SOCKET_DIR/wayland-0" "${'$'}XDG_RUNTIME_DIR/wayland-0"
                log_runtime_guest 'RUN: linked wayland socket into private runtime dir'

                unset SESSION_MANAGER

                export DISPLAY=:0
                export WAYLAND_DISPLAY=wayland-0
                export XDG_SESSION_TYPE=wayland
                export XDG_CURRENT_DESKTOP=XFCE
                export XCURSOR_PATH=/usr/share/icons
                export XCURSOR_THEME=default
                export QT_QPA_PLATFORM=xcb
                export GDK_BACKEND=x11
                export SDL_VIDEODRIVER=wayland
                export PULSE_SERVER=$PULSE_SERVER_VAL
                export GDK_SCALE=1
                export GDK_DPI_SCALE=$desktopScale
                export ELM_SCALE=$desktopScale
                export QT_AUTO_SCREEN_SCALE_FACTOR=1
                export QT_SCALE_FACTOR=$desktopScale
                export LIBGL_ALWAYS_SOFTWARE=1
                export GALLIUM_DRIVER=llvmpipe

                log_runtime_guest 'RUN: launching Xwayland :0...'
                mkdir -p /tmp/.X11-unix
                Xwayland :0 >/tmp/xwayland.log 2>&1 &
                XWAYLAND_PID=${'$'}!
                log_runtime_guest "RUN: Xwayland PID=${'$'}XWAYLAND_PID"

                x_wait=0
                while [ "${'$'}x_wait" -lt 15 ]; do
                    if [ -S "/tmp/.X11-unix/X0" ]; then
                        log_runtime_guest "RUN: confirmed X11 socket /tmp/.X11-unix/X0"
                        break
                    fi
                    sleep 1
                    x_wait=${'$'}((x_wait + 1))
                done
                if [ "${'$'}x_wait" -ge 15 ]; then
                    log_runtime_guest "WARN: X11 socket not found after 15s, continuing anyway"
                fi

                log_runtime_guest 'RUN: launching XFCE desktop session'

                if command -v startxfce4 >/dev/null 2>&1; then
                    log_runtime_guest "RUN: launching startxfce4 (X11 via Xwayland)..."
                    run_in_dbus_session startxfce4 >/tmp/xfce.log 2>&1
                else
                    log_runtime_guest "FATAL: startxfce4 not found in guest PATH"
                    exit 1
                fi
CHROOT_EOF
        """.trimIndent()
    }

    fun buildStopScript(filesDir: String, rootfsDir: String): String {
        return """
            #!/system/bin/sh
            INTERNAL_BB="$filesDir/bin/busybox"
            if [ -x "${'$'}INTERNAL_BB" ]; then
                BB="${'$'}INTERNAL_BB"
            else
                BB=$(command -v busybox || echo "busybox")
            fi

            unmount_safe() {
                target="$1"
                "${'$'}BB" umount -l "${'$'}target" 2>/dev/null || true
            }

            "${'$'}BB" chroot $rootfsDir /bin/bash -c '
                export HOME=/root
                export USER=root
                export LOGNAME=root
                export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
                command -v pkill >/dev/null 2>&1 && pkill -f startxfce4 || true
                command -v pkill >/dev/null 2>&1 && pkill -f xfce4-session || true
                command -v pkill >/dev/null 2>&1 && pkill -f labwc || true
                command -v pkill >/dev/null 2>&1 && pkill -f pulseaudio || true
                command -v pkill >/dev/null 2>&1 && pkill -f dbus-daemon || true
            ' || true

            pkill -f startxfce4 2>/dev/null || true
            pkill -f xfce4-session 2>/dev/null || true
            pkill -f labwc 2>/dev/null || true
            pkill -f pulseaudio 2>/dev/null || true
            pkill -f dbus-daemon 2>/dev/null || true

            unmount_safe $rootfsDir/dev/shm
            unmount_safe $rootfsDir/external_storage
            unmount_safe $rootfsDir/tmp
            unmount_safe $rootfsDir/dev/pts
            unmount_safe $rootfsDir/dev
            unmount_safe $rootfsDir/proc
            unmount_safe $rootfsDir/sys
        """.trimIndent()
    }
}
