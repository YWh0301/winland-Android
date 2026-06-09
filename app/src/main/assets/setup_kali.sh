#!/bin/bash
set -e

export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export DEBIAN_FRONTEND=noninteractive
export TMPDIR=/tmp
export TEMP=/tmp
export TMP=/tmp

mkdir -p /tmp /var/tmp /run
chmod 1777 /tmp /var/tmp || true

if ! command -v apt-get >/dev/null 2>&1; then
    echo "ERROR: apt-get not found in rootfs. Extraction appears incomplete/corrupted."
    exit 1
fi

if command -v dpkg-divert >/dev/null 2>&1; then
    dpkg-divert --remove /lib32 2>/dev/null || true
    dpkg-divert --remove /.lib32.usr-is-merged 2>/dev/null || true
fi

install_with_retry() {
    if ! apt-get -yq -o Dpkg::Options::="--force-confdef" -o Dpkg::Options::="--force-confold" install "$@"; then
        echo "WARN: apt install failed, attempting recovery..."
        dpkg --configure -a || true
        apt-get -yq -f install || true
        apt-get -yq -o Dpkg::Options::="--force-confdef" -o Dpkg::Options::="--force-confold" install "$@"
    fi
}

echo "INFO: Setting up Kali desktop"

mkdir -p /etc/apt/sources.list.d
echo 'deb [trusted=yes] http://ftp.de.debian.org/debian sid main' > /etc/apt/sources.list.d/debian-sid.list
mkdir -p /etc/apt/preferences.d
cat > /etc/apt/preferences.d/debian-pin << 'PIN_EOF'
Package: *
Pin: release a=sid
Pin-Priority: 100
PIN_EOF

apt-get -yq update

install_with_retry \
    sudo libwayland-client0 labwc xwayland dbus-x11 \
    pulseaudio pulseaudio-utils wlr-randr

install_with_retry xfce4 xfce4-terminal || true

mkdir -p /etc/xdg/labwc
cat > /etc/xdg/labwc/autostart <<'EOF_AUTOSTART'
#!/bin/bash
wlr-randr --output WL-1 --custom-mode 1080x2296
EOF_AUTOSTART
chmod +x /etc/xdg/labwc/autostart

RUNTIME_DIR="/tmp/xdg-runtime"
XDG_RUNTIME_DIR_VAL="/tmp"
PULSE_SERVER_VAL="unix:/tmp/pulse-socket"
mkdir -p "$RUNTIME_DIR"
chmod 700 "$RUNTIME_DIR"

cat >> /root/.bashrc <<'EOF_BASHRC'
export DISPLAY=:1
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=/tmp
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export SDL_VIDEODRIVER=wayland
export PULSE_SERVER=unix:/tmp/pulse-socket
EOF_BASHRC

cat >> /etc/profile <<'EOF_PROFILE'
export DISPLAY=:1
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=/tmp
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export SDL_VIDEODRIVER=wayland
export PULSE_SERVER=unix:/tmp/pulse-socket
EOF_PROFILE

echo "Kali setup finished."
