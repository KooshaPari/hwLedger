// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "HwLedger",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "HwLedger",
            targets: ["HwLedger"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "HwLedgerCore",
            path: "../xcframework/HwLedgerCore.xcframework"
        ),
        .target(
            name: "HwLedger",
            dependencies: ["HwLedgerCore"],
            path: "Sources",
            linkerSettings: [
                .linkedFramework("IOKit"),
                .linkedFramework("CoreFoundation"),
                .linkedLibrary("IOReport"),
            ]
        ),
        .testTarget(
            name: "HwLedgerTests",
            dependencies: ["HwLedger"],
            path: "Tests"
        ),
    ]
)
