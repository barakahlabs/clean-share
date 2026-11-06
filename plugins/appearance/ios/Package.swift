// swift-tools-version:5.5

import PackageDescription

let package = Package(
    name: "tauri-plugin-appearance",
    platforms: [
        .macOS(.v10_13),
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "tauri-plugin-appearance",
            type: .static,
            targets: ["tauri-plugin-appearance"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "tauri-plugin-appearance",
            dependencies: [
                .byName(name: "Tauri")
            ],
            path: "Sources")
    ]
)
