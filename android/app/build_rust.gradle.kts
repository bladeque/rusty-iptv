// Builds the Rust core for Android targets.
// Skipped automatically if cargo-ndk is not on PATH (e.g. Android Studio on a dev machine).
// In CI, cargo-ndk is always installed so the full native build runs.

val cargoNdkAvailable = try {
    Runtime.getRuntime().exec(arrayOf("cargo", "ndk", "--version"))
    true
} catch (e: Exception) {
    false
}

tasks.register<Exec>("buildRustCore") {
    enabled = cargoNdkAvailable
    workingDir = rootProject.file("../")
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-o", "${projectDir}/src/main/jniLibs",
        "build", "--release", "-p", "rusty-iptv-core"
    )
    doFirst {
        if (!cargoNdkAvailable) {
            println("cargo-ndk not found — skipping Rust native build (UI-only mode)")
        }
    }
}

tasks.named("preBuild") { dependsOn("buildRustCore") }
