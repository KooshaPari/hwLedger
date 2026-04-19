// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "HwLedgerApp",
    platforms: [
        .macOS(.v14)
    ],
    products: [
        .executable(
            name: "HwLedgerApp",
            targets: ["HwLedgerApp"]
        )
    ],
    dependencies: [
        .package(path: "../hwledger-swift")
    ],
    targets: [
        .executableTarget(
            name: "HwLedgerApp",
            dependencies: [
                .product(name: "HwLedger", package: "hwledger-swift")
            ],
            path: "Sources/HwLedgerApp"
        ),
        .testTarget(
            name: "HwLedgerAppTests",
            dependencies: [
                .product(name: "HwLedger", package: "hwledger-swift")
            ],
            path: "Tests/HwLedgerAppTests"
        )
    ]
)
