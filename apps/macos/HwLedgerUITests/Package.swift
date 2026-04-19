// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "HwLedgerUITests",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(
            name: "HwLedgerUITests",
            targets: ["HwLedgerUITestRunner"]
        )
    ],
    dependencies: [
        .package(url: "https://github.com/apple/swift-testing.git", from: "0.0.0")
    ],
    targets: [
        .target(
            name: "HwLedgerUITestHarness",
            dependencies: [],
            path: "Sources/Harness"
        ),
        .executableTarget(
            name: "HwLedgerUITestRunner",
            dependencies: [
                "HwLedgerUITestHarness",
                .product(name: "Testing", package: "swift-testing")
            ],
            path: "Sources/Runner"
        ),
        .testTarget(
            name: "HwLedgerUITests",
            dependencies: [
                "HwLedgerUITestHarness",
                .product(name: "Testing", package: "swift-testing")
            ],
            path: "Tests"
        )
    ]
)
