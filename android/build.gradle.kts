import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application") version "8.12.0"
    id("org.jetbrains.kotlin.android") version "2.2.20"
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.20"
}

val dotenv: Map<String, String> = file(".env").takeIf { it.isFile }
    ?.readLines()
    ?.map { it.trim() }
    ?.filter { it.isNotEmpty() && !it.startsWith("#") && "=" in it }
    ?.associate { line ->
        val (key, value) = line.split("=", limit = 2)
        key.trim() to value.trim().removeSurrounding("\"").removeSurrounding("'")
    }
    .orEmpty()

fun requiredEnvironmentVariable(name: String): String {
    return (System.getenv(name) ?: dotenv[name])?.takeIf(String::isNotBlank)
        ?: error("Required environment variable $name is missing or blank")
}

val appVersion: String? = providers.gradleProperty("appVersion").orNull

fun versionCodeOf(version: String): Int {
    val parts = version.split(".").map { it.toIntOrNull() }
    check(parts.size == 3 && parts.all { it != null && it in 0..999 }) {
        "appVersion must be MAJOR.MINOR.PATCH with each part in 0..999, got: $version"
    }
    val (major, minor, patch) = parts.map { it!! }
    return major * 1_000_000 + minor * 1_000 + patch
}

val keystorePath = requiredEnvironmentVariable("ANDROID_KEYSTORE_PATH")
val keystoreFile = file(keystorePath)
check(keystoreFile.isFile) {
    "ANDROID_KEYSTORE_PATH does not point to a file: $keystorePath"
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_11)
    }
}

android {
    namespace = "dev.ibylich.mpclipboard"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.ibylich.mpclipboard"
        minSdk = 26
        targetSdk = 35
        versionCode = appVersion?.let(::versionCodeOf) ?: 1
        versionName = appVersion ?: "0.0.0-dev"

        externalNativeBuild {
            cmake {
                arguments += "-DANDROID_STL=none"
            }
        }

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    signingConfigs {
        create("mandatory") {
            storeFile = keystoreFile
            storePassword = requiredEnvironmentVariable("ANDROID_KEYSTORE_PASSWORD")
            keyAlias = requiredEnvironmentVariable("ANDROID_KEY_ALIAS")
            keyPassword = requiredEnvironmentVariable("ANDROID_KEYSTORE_PASSWORD")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("mandatory")
        }
        debug {
            isMinifyEnabled = false
            signingConfig = signingConfigs.getByName("mandatory")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }

    buildFeatures {
        aidl = true
        buildConfig = true
        compose = true
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }
}

dependencies {
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation(platform("androidx.compose:compose-bom:2025.11.00"))
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.runtime:runtime")
    implementation("androidx.datastore:datastore-preferences:1.1.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
}
