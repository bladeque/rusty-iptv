// Run before every build: cargo ndk -t arm64-v8a -t armeabi-v7a -o app/src/main/jniLibs build --release
tasks.register<Exec>("buildRustCore") {
    workingDir = rootProject.file("../")
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-o", "${projectDir}/src/main/jniLibs",
        "build", "--release", "-p", "rusty-iptv-core"
    )
}

tasks.named("preBuild") { dependsOn("buildRustCore") }
