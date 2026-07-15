#!/usr/bin/env bash
set -euo pipefail
component=$(cd "$(dirname "$0")/.." && pwd)
ndk=${ANDROID_NDK_ROOT:-/opt/android-ndk}
api=${ANDROID_API:-31}
for tag in linux-x86_64 linux-aarch64; do
  [[ -x "$ndk/toolchains/llvm/prebuilt/$tag/bin/aarch64-linux-android${api}-clang" ]] && host_tag=$tag && break
done
: "${host_tag:?Android NDK clang not found under $ndk}"
toolchain="$ndk/toolchains/llvm/prebuilt/$host_tag"
build="$component/padputer/.build"
cross="$build/android-arm64.ini"
mkdir -p "$build"
cat >"$cross" <<EOF
[binaries]
c = '$toolchain/bin/aarch64-linux-android${api}-clang'
cpp = '$toolchain/bin/aarch64-linux-android${api}-clang++'
ar = '$toolchain/bin/llvm-ar'
strip = '$toolchain/bin/llvm-strip'
pkg-config = '/usr/bin/pkg-config'

[properties]
needs_exe_wrapper = true
sys_root = '$toolchain/sysroot'

[host_machine]
system = 'android'
cpu_family = 'aarch64'
cpu = 'armv8a'
endian = 'little'
EOF
xkb="$component/libxkbcommon"
if [[ -d "$xkb/build-android" ]]; then
  meson setup --wipe "$xkb/build-android" "$xkb" --cross-file "$cross" \
    --buildtype=release --auto-features=disabled -Denable-tools=false \
    -Denable-wayland=false -Denable-x11=false -Denable-xkbregistry=false \
    -Denable-bash-completion=false
else
  meson setup "$xkb/build-android" "$xkb" --cross-file "$cross" \
    --buildtype=release --auto-features=disabled -Denable-tools=false \
    -Denable-wayland=false -Denable-x11=false -Denable-xkbregistry=false \
    -Denable-bash-completion=false
fi
ninja -C "$xkb/build-android" libxkbcommon.so
rustup target list --installed | grep -qx aarch64-linux-android || {
  echo 'missing Rust target: rustup target add aarch64-linux-android' >&2
  exit 2
}
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$toolchain/bin/aarch64-linux-android${api}-clang"
export CC_aarch64_linux_android="$toolchain/bin/aarch64-linux-android${api}-clang"
export CXX_aarch64_linux_android="$toolchain/bin/aarch64-linux-android${api}-clang++"
(
  cd "$component/native"
  cargo build --release --lib --target aarch64-linux-android --features smithay_android
)
jni="$component/app/src/main/jniLibs/arm64-v8a"
mkdir -p "$jni"
install -m 0600 "$component/native/target/aarch64-linux-android/release/libuniffi_winland_core.so" "$jni/libuniffi_winland_core.so"
install -m 0600 "$xkb/build-android/libxkbcommon.so" "$jni/libxkbcommon.so"
install -m 0600 "$toolchain/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so" "$jni/libc++_shared.so"
file "$jni/libuniffi_winland_core.so" "$jni/libxkbcommon.so"
