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
            name: "SckBridge",
            dependencies: [],
            path: "Sources/SckBridge"
        ),
        .target(
            name: "HwLedgerGuiRecorder",
            dependencies: ["SckBridge"],
            path: "Sources/GuiRecorder"
        ),
        .target(
            name: "HwLedgerUITestHarness",
            dependencies: ["HwLedgerGuiRecorder"],
            path: "Sources/Harness"
        ),
        .target(
            name: "PhenotypeRecord",
            dependencies: [],
            path: "Sources/PhenotypeRecord"
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
                "PhenotypeRecord",
                .product(name: "Testing", package: "swift-testing")
            ],
            path: "Tests",
            exclude: ["PhenotypeRecordUnitTests"]
        ),
        .testTarget(
            name: "PhenotypeRecordUnitTests",
            dependencies: ["PhenotypeRecord"],
            path: "Tests/PhenotypeRecordUnitTests"
        )
    ]
)
