# Padputer bridge integration

This maintained fork is the Android/Smithay parent compositor used by the
top-level Padputer project. Read the parent repository's `HANDOFF.md` and
`WINLAND_BRIDGE_AUDIT.md` before changing manifest, process, mount or device
access behavior.

## Native compositor build

The upstream `build-arm64.sh` contains historical machine-specific `/root` and
old NDK paths and is not the canonical Padputer entry. Use:

```bash
./padputer/build-native.sh
```

Requirements: NDK r29 at `/opt/android-ndk` or `ANDROID_NDK_ROOT`, Meson/Ninja,
pkg-config, Rust with `aarch64-linux-android`, and the checked-out submodule
dependencies. The script generates a current NDK cross file, rebuilds
libxkbcommon, cross-builds the Rust core with `smithay_android`, and installs the
core, xkbcommon and NDK C++ runtime into `app/src/main/jniLibs/arm64-v8a/`.

Then package the debug APK from the submodule root:

```bash
ANDROID_HOME=${ANDROID_HOME:-/opt/android-sdk} \
ANDROID_SDK_ROOT=${ANDROID_SDK_ROOT:-/opt/android-sdk} \
JAVA_HOME=${JAVA_HOME:-/usr/lib/jvm/java-17-openjdk} \
  ./gradlew :app:assembleDebug
```

The bridge currently follows the inherited repository convention of tracking
core/xkbcommon JNI artifacts. A source change is not considered device-validated
until the native artifact is rebuilt, the parent submodule pointer is updated,
and the Smithay/Weston/Hyprland gates pass. The generated AHB presenter JNI is
ignored and supplied by the parent `components/ahb-presenter/build.sh`.

## Key Padputer behavior

- package `io.padputer.waylandbridge`, bridge-only launch extra;
- app-private XKB and client root;
- `wl_compositor` v6 and linux-dmabuf v4 feedback with verified LINEAR RGB only;
- complete `RenderFrame` boundaries and three-slot `FrameSourceBroker`;
- presentation-gated frame callbacks and retired buffer release;
- AHB generation, hot resize and fail-closed worker disconnect behavior;
- strict xdg-shell initial-configure validation with no app-id exception.

The former default-off Aquamarine compatibility branch was removed after the
patched client passed the strict 300-frame hardware gate.
