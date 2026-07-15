plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// The native compositor requires xkeyboard-config before it can publish a
// Wayland socket. Bridge-only builds package the host's architecture-neutral
// XKB data and copy it into the app-private bridge root at first launch.
val xkbSourceDir = providers.environmentVariable("XKB_CONFIG_ROOT")
    .orElse("/usr/share/X11/xkb")
val generatedXkbAssets = layout.buildDirectory.dir("generated/xkbAssets")
val stageXkbAssets by tasks.registering(Sync::class) {
    val source = file(xkbSourceDir.get())
    require(source.resolve("rules/evdev").isFile) {
        "xkeyboard-config not found at $source (install Arch package xkeyboard-config or set XKB_CONFIG_ROOT)"
    }
    from(source)
    into(generatedXkbAssets.map { it.dir("xkb") })
}

android {
    namespace = "com.winland.server"
    compileSdk = 34

    defaultConfig {
        applicationId = "io.padputer.waylandbridge"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "0.1-bridge-only"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }

        ndk {
            abiFilters += "arm64-v8a"
        }

    }

    signingConfigs {
        create("release") {
            storeFile = file("${rootProject.projectDir}/winland-release.keystore")
            storePassword = "winland123"
            keyAlias = "winland"
            keyPassword = "winland123"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("release")
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        compose = true
    }
    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.11"
    }
    sourceSets {
        getByName("main") {
            java.srcDirs("src/main/kotlin")
            jniLibs.srcDirs("src/main/jniLibs")
            assets.srcDir(generatedXkbAssets)
        }
    }
    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
        jniLibs {
            pickFirsts.add("**/libc++_shared.so")
            keepDebugSymbols += "**/*.so"
        }
    }
}

tasks.named("preBuild").configure {
    dependsOn(stageXkbAssets)
}

dependencies {
    // Termux terminal emulator
    implementation(project(":terminal-view"))

    implementation("androidx.compose.ui:ui-util")
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.lifecycle:lifecycle-service:2.7.0")
    implementation("androidx.activity:activity-compose:1.8.2")
    implementation(platform("androidx.compose:compose-bom:2024.02.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.navigation:navigation-compose:2.7.7")

    // OkHttp for downloading
    implementation("com.squareup.okhttp3:okhttp:4.12.0")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    // CameraX
    val cameraxVersion = "1.3.1"
    implementation("androidx.camera:camera-core:$cameraxVersion")
    implementation("androidx.camera:camera-camera2:$cameraxVersion")
    implementation("androidx.camera:camera-lifecycle:$cameraxVersion")

    // UniFFI and JNA dependencies
    implementation("net.java.dev.jna:jna:5.13.0@aar")
    implementation("com.github.topjohnwu.libsu:core:5.2.2")

    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation(platform("androidx.compose:compose-bom:2024.02.00"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
