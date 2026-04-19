// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "SckBridge",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .library(
            name: "SckBridge",
            type: .static,
            targets: ["SckBridge"]
        ),
    ],
    targets: [
        .target(
            name: "SckBridge",
            dependencies: [],
            path: "Sources/SckBridge",
            publicHeadersPath: ".",
            cSettings: [
                .headerSearchPath("."),
            ]
        ),
    ]
)
