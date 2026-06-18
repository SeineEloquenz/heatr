plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

// ===== CARGO =====

val cargoRoot: File = rootDir.parentFile // android/ → repo root
val jniLibsDir = "$projectDir/src/main/jniLibs"
val ndkTargets = listOf("-t", "arm64-v8a", "-t", "armeabi-v7a", "-t", "x86_64")

val cargoNdkDebug by tasks.registering(Exec::class) {
    description = "Build cargo ndk for debug"
    workingDir = cargoRoot
    commandLine(listOf("cargo", "ndk") + ndkTargets + listOf("-o", jniLibsDir, "build", "-p", "heatr-jni"))
}

val cargoNdkRelease by tasks.registering(Exec::class) {
    description = "Build cargo ndk for release"
    workingDir = cargoRoot
    commandLine(listOf("cargo", "ndk") + ndkTargets + listOf("-o", jniLibsDir, "build", "-p", "heatr-jni", "--release"))
}

// ===== /CARGO =====

android {
    dependenciesInfo {
        // Disables dependency metadata when building APKs.
        includeInApk = false
        // Disables dependency metadata when building Android App Bundles.
        includeInBundle = false
    }

    signingConfigs {
        create("release") {
            storeFile = file(System.getProperty("user.home") + "/work/_temp/keystore.jks")
            storePassword = System.getenv("KEYSTORE_PASSWORD")
            keyPassword = System.getenv("SIGNING_KEY_PASSWORD")
            keyAlias = System.getenv("SIGNING_KEY_ALIAS")
        }
    }

    namespace = "nz.eloque.heatr"
    compileSdk = 37

    defaultConfig {
        applicationId = "nz.eloque.heatr"
        minSdk = 28
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            jniLibs.directories.addAll(listOf("src/main/jniLibs"))
        }
    }
    buildToolsVersion = "37.0.0"
}

androidComponents {
    onVariants {
        it.sources.kotlin?.addStaticSourceDirectory("$cargoRoot/crates/heatr-jni/kotlin")
    }
}

afterEvaluate {
    tasks.named("mergeDebugJniLibFolders") { dependsOn(cargoNdkDebug) }
    tasks.named("mergeReleaseJniLibFolders") { dependsOn(cargoNdkRelease) }
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.core.ktx)
    implementation(libs.material)
    implementation(libs.androidx.material.icons.extended)
    implementation(libs.compose.kit)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}
