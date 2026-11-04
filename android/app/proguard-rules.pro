# UniFFI's Kotlin bindings call the Rust core through JNA, which finds
# classes, methods and fields by reflection.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class io.github.honeyslatt.opium.core.** { *; }
-dontwarn java.awt.**

# rustls-platform-verifier's Kotlin half is called from Rust through JNI.
-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }
