import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application") version "8.12.0"
    id("org.jetbrains.kotlin.android") version "2.2.20"
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.20"
}

fun requiredEnvironmentVariable(name: String): String {
    return System.getenv(name)?.takeIf(String::isNotBlank)
        ?: error("Required environment variable $name is missing or blank")
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
        versionCode = 1
        versionName = "0.1.0"

        externalNativeBuild {
            cmake {
                arguments += "-DANDROID_STL=c++_shared"
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
        compose = true
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }
}

dependencies {
    implementation("androidx.annotation:annotation:1.9.1")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation(platform("androidx.compose:compose-bom:2025.11.00"))
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.runtime:runtime")
    implementation("androidx.datastore:datastore-preferences:1.1.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
}
