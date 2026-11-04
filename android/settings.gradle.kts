pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        // The Kotlin half of rustls-platform-verifier, which the Rust core's
        // TLS uses to check certificates against Android's own store.
        maven("https://github.com/rustls/rustls-platform-verifier/raw/maven-archive/android-release-support/maven/")
    }
}

rootProject.name = "Opium"
include(":app")
