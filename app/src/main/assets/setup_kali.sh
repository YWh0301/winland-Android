#!/bin/bash
# Winland environment: installs XFCE on Kali in traditional (X11) mode
# without Wayland compositor dependency. Xwayland :0 provides the X11 display.
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
    sudo wget libwayland-client0 labwc xwayland dbus-x11 \
    pulseaudio pulseaudio-utils wlr-randr \
    fonts-noto-core fonts-kacst locales

install_with_retry xfce4 xfce4-terminal || true

echo "INFO: Installing GPU drivers (Vulkan + Mesa KGSL)..."
GPU_URL="https://github.com/eirkkk/winland-Android/releases/download/main/gpu.tar.gz"
GPU_TAR="/tmp/gpu.tar.gz"
if command -v wget >/dev/null 2>&1; then
    wget -q "$GPU_URL" -O "$GPU_TAR" || echo "WARN: wget failed"
elif command -v curl >/dev/null 2>&1; then
    curl -sL "$GPU_URL" -o "$GPU_TAR" || echo "WARN: curl failed"
fi
if [ -f "$GPU_TAR" ] && [ -s "$GPU_TAR" ]; then
    tar -xzf "$GPU_TAR" -C / --strip-components=1

    install_with_retry libxcb-keysyms1

    rm -f /usr/share/vulkan/icd.d/asahi_icd.json \
          /usr/share/vulkan/icd.d/broadcom_icd.json \
          /usr/share/vulkan/icd.d/gfxstream_vk_icd.json \
          /usr/share/vulkan/icd.d/intel_icd.json \
          /usr/share/vulkan/icd.d/lvp_icd.json \
          /usr/share/vulkan/icd.d/nouveau_icd.json \
          /usr/share/vulkan/icd.d/panfrost_icd.json \
          /usr/share/vulkan/icd.d/radeon_icd.json \
          /usr/share/vulkan/icd.d/virtio_icd.json \
          /usr/share/vulkan/icd.d/freedreno_icd.json

    rm -f /usr/lib/aarch64-linux-gnu/libvulkan_asahi.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_broadcom.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_gfxstream.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_intel.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_lvp.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_nouveau.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_panfrost.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_radeon.so \
          /usr/lib/aarch64-linux-gnu/libvulkan_virtio.so
    rm -f /usr/lib/aarch64-linux-gnu/libvulkan.so

    ldconfig 2>/dev/null || true

    ln -sf libvulkan.so.1 /usr/lib/aarch64-linux-gnu/libvulkan.so

    echo "INFO: GPU drivers installed and configured."
else
    echo "WARN: GPU tarball download failed. Will use software rendering."
fi

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
export DISPLAY=:0
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=/tmp
#export XCURSOR_PATH=/usr/share/icons
#export XCURSOR_THEME=default
#export QT_QPA_PLATFORM=wayland
#export GDK_BACKEND=wayland
#export SDL_VIDEODRIVER=wayland
export GDK_BACKEND=x11
export QT_QPA_PLATFORM=xcb
export PULSE_SERVER=unix:/tmp/pulse-socket
EOF_BASHRC

cat >> /etc/profile <<'EOF_PROFILE'
export DISPLAY=:0
export PULSE_SERVER=127.0.0.1
export WAYLAND_DISPLAY=wayland-0
export XDG_RUNTIME_DIR=/tmp
#export XCURSOR_PATH=/usr/share/icons
#export XCURSOR_THEME=default
#export QT_QPA_PLATFORM=wayland
#export GDK_BACKEND=wayland
#export SDL_VIDEODRIVER=wayland
export GDK_BACKEND=x11
export QT_QPA_PLATFORM=xcb
export PULSE_SERVER=unix:/tmp/pulse-socket
EOF_PROFILE

echo "INFO: Generating Arabic locale..."
locale-gen ar_SA.UTF-8 || true
echo 'LANG=ar_SA.UTF-8' >> /etc/default/locale 2>/dev/null || true

echo "INFO: Installing Firefox from Mozilla official repo"
install -d /etc/apt/keyrings
wget -q https://mozilla.org -O- | tee /etc/apt/keyrings/packages.mozilla.org.asc > /dev/null
echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.asc] https://mozilla.org mozilla main" | tee /etc/apt/sources.list.d/mozilla.list > /dev/null
cat > /etc/apt/preferences.d/mozilla <<'EOF'
Package: firefox*
Pin: origin packages.mozilla.org
Pin-Priority: 1001
EOF
apt-get -yq update || true
install_with_retry firefox || true

echo "Kali setup finished."
