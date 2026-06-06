#!/bin/bash
# Desktop Setup for Winland Server chroot

set -e

export PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
export DEBIAN_FRONTEND=noninteractive
export TMPDIR=/tmp
export TEMP=/tmp
export TMP=/tmp

# Ensure temporary/runtime dirs exist inside chroot for dpkg triggers
mkdir -p /tmp /var/tmp /run
chmod 1777 /tmp /var/tmp || true

if ! command -v apt-get >/dev/null 2>&1; then
    echo "ERROR: apt-get not found in rootfs. Extraction appears incomplete/corrupted."
    exit 1
fi

apt-get -yq update

install_desktop_packages() {
    apt-get -yq \
        -o Dpkg::Options::="--force-confdef" \
        -o Dpkg::Options::="--force-confold" \
        install \
        sudo \
        libwayland-client0 \
        labwc \
        xwayland \
        dbus-x11 \
        pulseaudio \
        pulseaudio-utils \
        wlr-randr
}

enable_xfce_wayland_experimental() {
    echo "INFO: enabling Xfce Wayland experimental repo"
    apt-get -yq install software-properties-common || true
    add-apt-repository -y ppa:xubuntu-dev/experimental || true
    apt-get -yq update || true
    apt-get -yq \
        -o Dpkg::Options::="--force-confdef" \
        -o Dpkg::Options::="--force-confold" \
        install \
        labwc xfce4* xfce4-goodies xfce4-terminal || true
}

if ! install_desktop_packages; then
    echo "WARN: apt install failed, attempting recovery..."
    dpkg --configure -a || true
    apt-get -yq -f install || true
    install_desktop_packages
fi

enable_xfce_wayland_experimental

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

export DISPLAY=:0
export DISPLAY=:1
export WAYLAND_DISPLAY=wayland-0
export PULSE_SERVER=127.0.0.1
export XDG_RUNTIME_DIR="$XDG_RUNTIME_DIR_VAL"
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export SDL_VIDEODRIVER=wayland
export PULSE_SERVER="$PULSE_SERVER_VAL"

cat >> /root/.bashrc <<EOF_BASHRC
export DISPLAY=:0
export DISPLAY=:1
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR_VAL
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export SDL_VIDEODRIVER=wayland
export PULSE_SERVER=$PULSE_SERVER_VAL
EOF_BASHRC

cat >> /etc/profile <<EOF_PROFILE
export DISPLAY=:0
export DISPLAY=:1
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=$XDG_RUNTIME_DIR_VAL
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export SDL_VIDEODRIVER=wayland
export PULSE_SERVER=$PULSE_SERVER_VAL
EOF_PROFILE

echo "Setup Finished. Xfce Wayland (LabWC) environment is ready natively."
