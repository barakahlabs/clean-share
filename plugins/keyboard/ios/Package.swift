// swift-tools-version:5.5

import PackageDescription

let package = Package(
    name: "tauri-plugin-keyboard",
    platforms: [
        .macOS(.v10_13),
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "tauri-plugin-keyboard",
            type: .static,
            targets: ["tauri-plugin-keyboard"]),
    ],
    dependencies: [
        .package(name: "Tauri", path: "../.tauri/tauri-api")
    ],
    targets: [
        .target(
            name: "tauri-plugin-keyboard",
            dependencies: [
                .byName(name: "Tauri")
            ],
            path: "Sources")
    ]
)
