plugins {
    id("com.android.library") version "9.4.1"
}

fun requiredEnvironmentVariable(name: String): String {
    return System.getenv(name)?.takeIf(String::isNotBlank)
        ?: error("Required environment variable $name is missing or blank")
}

android {
    namespace = "dev.ibylich.mpclipboard"
    compileSdk = 37
    ndkVersion = requiredEnvironmentVariable("ANDROID_NDK_VERSION")

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")

        externalNativeBuild {
            cmake {
                arguments += "-DANDROID_STL=none"
            }
        }

        ndk {
            abiFilters += listOf("arm64-v8a")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = requiredEnvironmentVariable("ANDROID_CMAKE_VERSION")
        }
    }
}
