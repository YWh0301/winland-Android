#!/bin/bash
# Build script for Winland Server on ARM64 - Modern Meson Compile Version

set -e

# Load Rust environment
source "$HOME/.cargo/env"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}🔨 Winland Server - ARM64 Build (Modern Compile Mode)${NC}"
echo "==========================================="

# Check Rust installation
echo -e "${YELLOW}✓ Checking Rust environment...${NC}"
rustc --version
cargo --version

# Check Android targets
echo -e "${YELLOW}✓ Checking ARM64 target...${NC}"
if ! rustup target list | grep -q "aarch64-linux-android (installed)"; then
    echo -e "${RED}✗ ARM64 target not installed${NC}"
    echo "Installing ARM64 target..."
    rustup target add aarch64-linux-android
fi

cd "$(dirname "$0")"
PROJECT_ROOT="$(pwd)"

# =====================================================================
# 🧠 Smart check and direct Meson compile for libxkbcommon
# =====================================================================
XKB_DIR="$PROJECT_ROOT/libxkbcommon"
XKB_OUT_SO="$XKB_DIR/build-android/libxkbcommon.so"
echo -e "${YELLOW}🔍 Checking libxkbcommon.so state...${NC}"
if [ -f "$XKB_OUT_SO" ] && [ -s "$XKB_OUT_SO" ]; then
    echo -e "${GREEN}⚡ Smart Skip: libxkbcommon.so is already built and valid. Skipping C-build pipeline!${NC}"
else
    echo -e "${YELLOW}⚠️ libxkbcommon.so not found or invalid. Compiling target library...${NC}"

    cd "$XKB_DIR"

    if [ -d "build-android" ]; then
        meson setup --reconfigure build-android \
            --cross-file android-arm64.ini \
            --auto-features=disabled \
            -Denable-tools=false \
            -Denable-x11=false \
            -Denable-xkbregistry=false \
            -Denable-bash-completion=false > /dev/null 2>&1
    else
        meson setup build-android \
            --cross-file android-arm64.ini \
            --auto-features=disabled \
            -Denable-tools=false \
            -Denable-x11=false \
            -Denable-xkbregistry=false \
            -Denable-bash-completion=false
    fi

    ninja -C build-android libxkbcommon.so

    cd "$PROJECT_ROOT"
    echo -e "${GREEN}✓ libxkbcommon.so built successfully!${NC}"
fi
# =====================================================================

# 📟 Build Terminal Emulator JNI library (libtermux.so) - fixed path
# =====================================================================
echo -e "${YELLOW}📟 Building Terminal Emulator JNI library (libtermux.so)...${NC}"
cd "$PROJECT_ROOT/terminal-emulator/src/main"

/opt/android/ndk/ndk-build NDK_PROJECT_PATH=. clean
/opt/android/ndk/ndk-build \
    NDK_PROJECT_PATH=. \
    APP_ABI=arm64-v8a \
    APP_PLATFORM=android-31

echo -e "${GREEN}✓ libtermux.so built successfully!${NC}"
cd "$PROJECT_ROOT"
# =====================================================================

# Set Android compilers for Rust
export CC_aarch64_linux_android=/opt/android/ndk/toolchains/llvm/prebuilt/linux-aarch64/bin/aarch64-linux-android31-clang
export CXX_aarch64_linux_android=/opt/android/ndk/toolchains/llvm/prebuilt/linux-aarch64/bin/aarch64-linux-android31-clang++

# Build Rust native library
echo -e "${YELLOW}📦 Building Rust native library (ARM64)...${NC}"
cd native
rm -f target/aarch64-linux-android/release/libuniffi_winland_core.so
cargo build \
    --release \
    --lib \
    --target aarch64-linux-android \
    --features smithay_android \
    --jobs 4

echo -e "${GREEN}✓ Rust build completed!${NC}"

# Copy native libraries to JNI directory
echo -e "${YELLOW}📦 Copying native libraries to JNI...${NC}"
mkdir -p ../app/src/main/jniLibs/arm64-v8a
rm -rf /root/winland-Android/app/build/outputs/apk/debug/*

# Copy Rust core library
cp target/aarch64-linux-android/release/libuniffi_winland_core.so /root/winland-Android/app/src/main/jniLibs/arm64-v8a

# Remove old APK from sdcard (optional, ignore if not exists)
rm -f /sdcard/app-debug.apk

# Copy terminal library from the new correct path
cp ../terminal-emulator/src/main/libs/arm64-v8a/libtermux.so /root/winland-Android/app/src/main/jniLibs/arm64-v8a

echo -e "${GREEN}✓ All native libraries copied to target!${NC}"

# Build Android APK
echo -e "${YELLOW}🔨 Building Android APK...${NC}"
cd ..

export ANDROID_HOME=/opt/android/sdk
export ANDROID_SDK_ROOT=/opt/android/sdk
export ANDROID_NDK_ROOT=/opt/android/ndk

./gradlew clean assembleDebug

echo -e "${GREEN}✓ Build completed successfully!${NC}"
echo "==========================================="
echo -e "${GREEN}APK location: ${NC}app/build/outputs/apk/"

cp -r /root/winland-Android/app/build/outputs/apk/debug/* /sdcard 2>/dev/null || echo "⚠️ Could not copy APK to /sdcard"
