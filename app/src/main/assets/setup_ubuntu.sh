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
apt-get -yq install software-properties-common || true

echo "INFO: Adding GPU drivers PPA (kisak/turtle)..."
add-apt-repository -y ppa:kisak/turtle || true

echo "INFO: Adding XFCE experimental PPA..."
add-apt-repository -y ppa:xubuntu-dev/experimental || true

apt-get -yq update
apt-get -yq upgrade || true

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
        wlr-randr \
        fonts-noto \
        locales \
        xsel \
        wl-clipboard
}

enable_xfce_wayland_experimental() {
    echo "INFO: installing XFCE"
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

echo "INFO: Installing GPU drivers (Mesa GPU deb)..."
GPU_URL="https://github.com/eirkkk/winland-Android/releases/download/main/mesa-gpu_25.2.8_arm64.deb"
GPU_DEB="/tmp/mesa-gpu_25.2.8_arm64.deb"
if command -v wget >/dev/null 2>&1; then
    wget -q "$GPU_URL" -O "$GPU_DEB" || echo "WARN: wget failed"
elif command -v curl >/dev/null 2>&1; then
    curl -sL "$GPU_URL" -o "$GPU_DEB" || echo "WARN: curl failed"
fi
if [ -f "$GPU_DEB" ] && [ -s "$GPU_DEB" ]; then
    dpkg -i "$GPU_DEB" || apt-get -yq -f install || true

    apt-get -yq install libxcb-keysyms1 || true

    echo "INFO: GPU drivers installed and configured."
else
    echo "WARN: GPU deb download failed. Will use software rendering."
fi

echo "INFO: Generating Arabic locale..."
locale-gen ar_SA.UTF-8 || true

mkdir -p /etc/xdg/labwc

cat > /etc/xdg/labwc/environment <<'EOF_ENV'
# Keyboard layout: US English primary, Arabic secondary
# Toggle with Shift+CapsLock
XKB_DEFAULT_LAYOUT=us,ara
XKB_DEFAULT_OPTIONS=grp:shift_caps_toggle,grp_led:scroll
# Cursor
XCURSOR_THEME=default
XCURSOR_SIZE=24
# Java non-reparenting for XWayland
_JAVA_AWT_WM_NONREPARENTING=1
EOF_ENV

cat > /etc/xdg/labwc/rc.xml <<'EOF_RCXML'
<?xml version="1.0"?>
<labwc_config>
  <core>
    <decoration>server</decoration>
    <gap>0</gap>
    <adaptiveSync>no</adaptiveSync>
    <allowTearing>no</allowTearing>
    <reuseOutputMode>no</reuseOutputMode>
  </core>

  <placement>
    <policy>center</policy>
  </placement>

  <theme>
    <name>Clearlooks</name>
    <cornerRadius>6</cornerRadius>
    <keepBorder>yes</keepBorder>
    <font place="ActiveWindow">
      <name>sans</name>
      <size>10</size>
    </font>
    <font place="InactiveWindow">
      <name>sans</name>
      <size>10</size>
    </font>
    <font place="MenuItem">
      <name>sans</name>
      <size>10</size>
    </font>
    <font place="OnScreenDisplay">
      <name>sans</name>
      <size>10</size>
    </font>
  </theme>

  <windowSwitcher show="yes" preview="yes" outlines="yes">
    <fields>
      <field content="type" width="25%" />
      <field content="trimmed_identifier" width="25%" />
      <field content="title" width="50%" />
    </fields>
  </windowSwitcher>

  <resistance>
    <screenEdgeStrength>20</screenEdgeStrength>
    <windowEdgeStrength>20</windowEdgeStrength>
  </resistance>

  <resize popupShow="Never" />

  <focus>
    <followMouse>no</followMouse>
    <followMouseRequiresMovement>yes</followMouseRequiresMovement>
    <raiseOnFocus>no</raiseOnFocus>
  </focus>

  <snapping>
    <range>5</range>
    <topMaximize>yes</topMaximize>
    <notifyClient>always</notifyClient>
  </snapping>

  <desktops>
    <popupTime>1000</popupTime>
    <names>
      <name>Workspace 1</name>
      <name>Workspace 2</name>
      <name>Workspace 3</name>
      <name>Workspace 4</name>
    </names>
  </desktops>

  <keyboard>
    <numlock>on</numlock>
    <layoutScope>global</layoutScope>
    <repeatRate>25</repeatRate>
    <repeatDelay>600</repeatDelay>
    <default />
    <!-- Override default terminal: use xfce4-terminal -->
    <keybind key="W-Return">
      <action name="Execute" command="xfce4-terminal" />
    </keybind>
    <!-- XFCE run dialog -->
    <keybind key="A-F2">
      <action name="Execute" command="xfce4-appfinder" />
    </keybind>
    <!-- Screenshot -->
    <keybind key="Print">
      <action name="Execute" command="xfce4-screenshooter" />
    </keybind>
    <!-- Lock screen -->
    <keybind key="C-A-L">
      <action name="Execute" command="xflock4" />
    </keybind>
  </keyboard>

  <mouse>
    <doubleClickTime>500</doubleClickTime>
    <scrollFactor>1.0</scrollFactor>
    <default />
    <context name="Root">
      <mousebind button="Left" action="Press">
        <action name="ShowMenu" menu="root-menu" />
      </mousebind>
      <mousebind button="Right" action="Press">
        <action name="ShowMenu" menu="root-menu" />
      </mousebind>
      <mousebind direction="Up" action="Scroll">
        <action name="GoToDesktop" to="left" wrap="yes" />
      </mousebind>
      <mousebind direction="Down" action="Scroll">
        <action name="GoToDesktop" to="right" wrap="yes" />
      </mousebind>
    </context>
  </mouse>

  <touch deviceName="" mapToOutput="" />

  <libinput>
    <device category="default">
      <tap>yes</tap>
      <naturalScroll>yes</naturalScroll>
      <leftHanded>no</leftHanded>
      <middleEmulation>yes</middleEmulation>
      <disableWhileTyping>yes</disableWhileTyping>
      <clickMethod>clickfinger</clickMethod>
    </device>
    <device category="touch">
      <tap>yes</tap>
    </device>
  </libinput>
</labwc_config>
EOF_RCXML

cat > /etc/xdg/labwc/autostart <<'EOF_AUTOSTART'
#!/bin/bash
# Set output mode to match Android display
wlr-randr --output WL-1 --custom-mode 1080x2296
EOF_AUTOSTART
chmod +x /etc/xdg/labwc/autostart

mkdir -p /etc/xdg/menus
cat > /etc/xdg/menus/labwc-applications.menu <<'EOF_MENU'
<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
 "http://www.freedesktop.org/DTs/menu-1.0.dtd">
<Menu>
  <Name>Applications</Name>
  <DefaultAppDirs/>
  <DefaultDirectoryDirs/>
</Menu>
EOF_MENU

RUNTIME_DIR="/tmp/xdg-runtime"
XDG_RUNTIME_DIR_VAL="/tmp"
PULSE_SERVER_VAL="unix:/tmp/pulse-socket"
mkdir -p "$RUNTIME_DIR"
chmod 700 "$RUNTIME_DIR"

cat >> /root/.bashrc <<'EOF_BASHRC'
export DISPLAY=:0
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=XFCE
export WAYLAND_DISPLAY=wayland-0
export PULSE_SERVER=unix:/tmp/pulse-runtime/native
export XDG_RUNTIME_DIR=/tmp
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export XKB_DEFAULT_LAYOUT=us,ara
export XKB_DEFAULT_OPTIONS=grp:shift_caps_toggle,grp_led:scroll
export QT_QPA_PLATFORM=xcb
export GDK_BACKEND=x11
EOF_BASHRC

mkdir -p /etc/profile.d
cat > /etc/profile.d/winland.sh <<'EOF_PROFILE'
export DISPLAY=:0
export XDG_SESSION_TYPE=wayland
export XDG_CURRENT_DESKTOP=XFCE
export WAYLAND_DISPLAY=wayland-0
export PULSE_SERVER=unix:/tmp/pulse-runtime/native
export XDG_RUNTIME_DIR=/tmp
export XCURSOR_PATH=/usr/share/icons
export XCURSOR_THEME=default
export XKB_DEFAULT_LAYOUT=us,ara
export XKB_DEFAULT_OPTIONS=grp:shift_caps_toggle,grp_led:scroll
export QT_QPA_PLATFORM=xcb
export GDK_BACKEND=x11
EOF_PROFILE
chmod +x /etc/profile.d/winland.sh

echo "INFO: Adding Mozilla Firefox repository..."
apt-get -yq install wget gpg 2>/dev/null || true
install -d /etc/apt/keyrings
wget -q https://packages.mozilla.org/apt/repo-signing-key.gpg -O- | gpg --dearmor > /etc/apt/keyrings/packages.mozilla.org.gpg 2>/dev/null
echo "deb [signed-by=/etc/apt/keyrings/packages.mozilla.org.gpg] https://packages.mozilla.org/apt mozilla main" | tee /etc/apt/sources.list.d/mozilla.list > /dev/null
cat << 'EOF' > /etc/apt/preferences.d/mozilla
Package: firefox*
Pin: origin packages.mozilla.org
Pin-Priority: 1001
EOF
apt-get update
apt-get -yq install firefox || true

echo "Setup Finished. Xfce X11 (LabWC) environment is ready natively."
