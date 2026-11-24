import java.util.Properties
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 37
    namespace = "app.unisono.tuner"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "app.unisono.tuner"
        minSdk = 26
        targetSdk = 37
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        create("release") {
            val keystorePath = System.getenv("UNISONO_KEYSTORE")
            val storePassword = System.getenv("UNISONO_STORE_PASSWORD")
            val keyAlias = System.getenv("UNISONO_KEY_ALIAS")
            val keyPassword = System.getenv("UNISONO_KEY_PASSWORD")

            if (!keystorePath.isNullOrBlank() &&
                !storePassword.isNullOrBlank() &&
                !keyAlias.isNullOrBlank() &&
                !keyPassword.isNullOrBlank()
            ) {
                storeFile = file(keystorePath)
                this.storePassword = storePassword
                this.keyAlias = keyAlias
                this.keyPassword = keyPassword
            }
        }
    }
    buildTypes {
        getByName("debug") {
            applicationIdSuffix = ".debug"
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {
                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            signingConfig = signingConfigs.getByName("release")
            optimization {
               enable = true
            }
            proguardFiles(
                *fileTree(".") {
                  include("**/*.pro")
                  exclude("build/**")
                }.files.toTypedArray()
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    buildFeatures {
        buildConfig = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget = JvmTarget.JVM_1_8
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = file("tauri.build.gradle.kts"))

gradle.taskGraph.whenReady {
    val hasReleaseTask = allTasks.any { task ->
        task.name.contains("Release", ignoreCase = true)
    }
    if (hasReleaseTask) {
        val keystorePath = System.getenv("UNISONO_KEYSTORE")
        val storePassword = System.getenv("UNISONO_STORE_PASSWORD")
        val keyAlias = System.getenv("UNISONO_KEY_ALIAS")
        val keyPassword = System.getenv("UNISONO_KEY_PASSWORD")

        val missing = mutableListOf<String>()
        if (keystorePath.isNullOrBlank()) missing.add("UNISONO_KEYSTORE")
        if (storePassword.isNullOrBlank()) missing.add("UNISONO_STORE_PASSWORD")
        if (keyAlias.isNullOrBlank()) missing.add("UNISONO_KEY_ALIAS")
        if (keyPassword.isNullOrBlank()) missing.add("UNISONO_KEY_PASSWORD")

        if (missing.isNotEmpty()) {
            throw GradleException(
                "Release signing requires environment variables: ${missing.joinToString(", ")}. " +
                "Set UNISONO_KEYSTORE, UNISONO_STORE_PASSWORD, UNISONO_KEY_ALIAS, UNISONO_KEY_PASSWORD to build release APK."
            )
        }
    }
}
