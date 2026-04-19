---
title: FFI Survey — Rust ↔ Native Language Bindings
description: UniFFI vs cbindgen vs csbindgen vs cxx-qt vs Slint for SwiftUI, WinUI, and Qt frontends. Tool selection per platform.
brief_id: 7
status: archived
date: 2026-04-18
sources:
  - url: https://github.com/mozilla/uniffi-rs
    title: UniFFI — Mozilla's FFI Binding Generator
  - url: https://github.com/mozilla/cbindgen
    title: cbindgen — C Header Generator
  - url: https://github.com/mozilla/csbindgen
    title: csbindgen — C# Binding Generator
  - url: https://github.com/kdab/cxx-qt
    title: cxx-qt — Rust-Qt Integration (KDAB)
  - url: https://slint.dev/
    title: Slint — Declarative UI Framework
---

# FFI Survey — Rust ↔ Native Language Bindings

## Overview

hwLedger ships three native frontends (macOS/Windows/Linux) over a shared Rust core. Each platform requires a different FFI strategy:

| Platform | Frontend | FFI Tool | Async | Callbacks | License | Maturity |
|----------|----------|----------|-------|-----------|---------|----------|
| macOS | SwiftUI | **UniFFI** | Native | Yes (async) | MIT | Prod |
| Windows | WinUI 3 / C# | **csbindgen** | Native (.NET async) | Yes | MIT | Prod |
| Linux | Qt 6 / QML | **cxx-qt** | Tokio bridge | Qt signals | LGPL/MIT | Prod |
| All (escape hatch) | Slint | **Slint built-in** | Native | Native | MIT | Beta |

## 1. macOS: UniFFI + cargo-xcframework

### Why UniFFI?

**UniFFI** (Mozilla) is the gold standard for Rust-to-Swift interop. Signal, 1Password, and Mozilla Firefox all converged here.

**Strengths**:
- Swift async/await native (no .call wrappers).
- Automatic `Result<T>` → throws conversion.
- Callback traits for streaming (slider updates, token generation).
- XCFramework packaging automatic (one binary for simulator + device + architectures).

**Weaknesses**:
- Learning curve (UDL file syntax).
- Procedural macro overhead (slower incremental builds).

### Workflow

```
hwledger-core (Rust lib)
    ↓ UniFFI UDL file
        ↓ uniffi code generator
            ↓ Swift module
                ↓ XCFramework
                    ↓ Xcode import
                        ↓ SwiftUI app
```

### UDL File (Interface Definition Language)

`crates/hwledger-ffi/src/hwledger.udl`:

```
// hwledger.udl
namespace hwledger {
  sequence<u8> ByteSlice;
};

dictionary MemoryPrediction {
  u64 weights_mb;
  u64 kv_cache_mb;
  u64 runtime_overhead_mb;
  u64 total_mb;
};

[Async]
interface Planner {
  constructor();
  
  [Async]
  MemoryPrediction estimate_vram(
    string model_id,
    u32 sequence_length,
    u32 batch_size,
    string quantization
  );
};

callback interface TokenStreamListener {
  void on_token(string token);
  void on_complete(string full_text);
};

[Async]
interface InferenceRunner {
  [Async]
  void run_inference(
    string model_path,
    string prompt,
    TokenStreamListener listener
  );
};
```

### Rust Implementation

`crates/hwledger-ffi/src/lib.rs`:

```rust
#![allow(unsafe_code)]

uniffi::setup_scaffolding!();

pub struct Planner {
    core: Arc<hwledger_core::Planner>,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            core: Arc::new(hwledger_core::Planner::new()),
        }
    }

    pub async fn estimate_vram(
        &self,
        model_id: String,
        sequence_length: u32,
        batch_size: u32,
        quantization: String,
    ) -> Result<MemoryPrediction, PlannerError> {
        let estimate = self.core
            .estimate(hwledger_core::EstimateRequest {
                model_id,
                seq_len: sequence_length as usize,
                batch_size: batch_size as usize,
                quantization,
            })
            .await?;

        Ok(MemoryPrediction {
            weights_mb: estimate.weights / (1024 * 1024),
            kv_cache_mb: estimate.kv_cache / (1024 * 1024),
            runtime_overhead_mb: estimate.runtime / (1024 * 1024),
            total_mb: estimate.total() / (1024 * 1024),
        })
    }
}

pub trait TokenStreamListener: Send + Sync {
    fn on_token(&self, token: String);
    fn on_complete(&self, full_text: String);
}

pub struct InferenceRunner;

impl InferenceRunner {
    pub async fn run_inference(
        model_path: String,
        prompt: String,
        listener: Arc<dyn TokenStreamListener>,
    ) -> Result<(), InferenceError> {
        let mut runner = hwledger_inference::Runner::new(&model_path)?;
        
        for token in runner.generate(&prompt).await? {
            listener.on_token(token.clone());
        }
        
        let full = runner.full_output().to_string();
        listener.on_complete(full);
        
        Ok(())
    }
}
```

### Build Integration

`Cargo.toml`:

```toml
[lib]
name = "hwledger_ffi"
crate-type = ["staticlib", "cdylib"]

[dependencies]
uniffi = { version = "0.27", features = ["build", "cli"] }
tokio = { version = "1", features = ["full"] }
```

Build script (`build.rs`):

```rust
use std::env;
use std::path::PathBuf;

fn main() {
    uniffi::generate_scaffolding("crates/hwledger-ffi/src/hwledger.udl")
        .expect("Failed to generate UniFFI scaffolding");

    // XCFramework generation (via cargo-xcframework)
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search={}", out_dir);
}
```

### XCFramework Packaging

```bash
cargo install cargo-xcframework

cargo xcframework \
  --manifest-path crates/hwledger-ffi/Cargo.toml \
  --release \
  --out apps/macos/Frameworks/hwledger.xcframework
```

Result: `hwledger.xcframework` contains binaries for:
- `arm64-apple-macos` (native Apple Silicon)
- `x86_64-apple-macos` (Intel, for cross-dev)
- `arm64-apple-ios-sim` (simulator)

### SwiftUI Integration

`apps/macos/Sources/HwLedgerApp.swift`:

```swift
import SwiftUI
import HwledgerFFI

@main
struct HwledgerApp: App {
    @State private var planner = Planner()

    var body: some Scene {
        WindowGroup {
            ContentView(planner: planner)
        }
    }
}

struct ContentView: View {
    let planner: Planner
    @State private var modelId = "mistral-7b"
    @State private var seqLen: UInt32 = 4096
    @State private var estimatedVram: MemoryPrediction?
    @State private var isLoading = false

    var body: some View {
        VStack {
            TextField("Model ID", text: $modelId)
            Slider(value: $seqLen.cgFloat, in: 128...32768)
            
            Button("Estimate VRAM") {
                isLoading = true
                Task {
                    do {
                        estimatedVram = try await planner.estimateVram(
                            modelId: modelId,
                            sequenceLength: seqLen,
                            batchSize: 1,
                            quantization: "fp16"
                        )
                    } catch {
                        print("Error: \(error)")
                    }
                    isLoading = false
                }
            }

            if let vram = estimatedVram {
                Text("Total VRAM: \(vram.totalMb) MB")
            }
        }
    }
}
```

## 2. Windows: csbindgen + WinUI 3 / .NET 9

**csbindgen** generates C# bindings from Rust. Uses C#'s native async/await and AOT compilation.

### Workflow

```
hwledger-core (Rust lib)
    ↓ cbindgen (for C header)
        ↓ csbindgen (C header → C# bindings)
            ↓ C# P/Invoke + NativeAOT
                ↓ WinUI 3 app
```

### cbindgen for C Header

`crates/hwledger-ffi/cbindgen.toml`:

```toml
language = "c"
namespace = "hwledger"
include_guard = "HWLEDGER_H"
```

Generate header:

```bash
cbindgen crates/hwledger-ffi --output apps/windows/HwledgerFFI.h
```

Result: `HwledgerFFI.h` with C-compatible function signatures.

### csbindgen Configuration

`csbindgen.toml`:

```toml
[bindgen]
input_bindgen_file = "apps/windows/HwledgerFFI.h"
output_file_path = "apps/windows/HwledgerFFI/Interop.g.cs"
namespace = "HwledgerFFI.Interop"
use_runtime_marshalling = true
```

Generate C# bindings:

```bash
csbindgen
```

### WinUI 3 / C# Integration

`apps/windows/HwledgerFFI/Planner.cs`:

```csharp
using System.Runtime.InteropServices;
using HwledgerFFI.Interop;

namespace HwledgerFFI;

public partial class Planner : IDisposable
{
    private IntPtr _handle;

    public Planner()
    {
        _handle = HwledgerInterop.planner_new();
    }

    public async Task<MemoryPrediction> EstimateVramAsync(
        string modelId,
        uint sequenceLength,
        uint batchSize,
        string quantization)
    {
        var result = await Task.Run(() =>
            HwledgerInterop.planner_estimate_vram(
                _handle,
                modelId,
                sequenceLength,
                batchSize,
                quantization
            )
        );

        return new MemoryPrediction
        {
            WeightsMb = result.weights_mb,
            KvCacheMb = result.kv_cache_mb,
            RuntimeOverheadMb = result.runtime_overhead_mb,
            TotalMb = result.total_mb,
        };
    }

    public void Dispose()
    {
        if (_handle != IntPtr.Zero)
        {
            HwledgerInterop.planner_free(_handle);
            _handle = IntPtr.Zero;
        }
    }
}

public class MemoryPrediction
{
    public ulong WeightsMb { get; set; }
    public ulong KvCacheMb { get; set; }
    public ulong RuntimeOverheadMb { get; set; }
    public ulong TotalMb { get; set; }
}
```

### WinUI XAML UI

`apps/windows/Views/PlannerPage.xaml.cs`:

```csharp
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace HwledgerApp.Views;

public sealed partial class PlannerPage : Page
{
    private Planner _planner = new();

    public PlannerPage()
    {
        InitializeComponent();
    }

    private async void EstimateButton_Click(object sender, RoutedEventArgs e)
    {
        var prediction = await _planner.EstimateVramAsync(
            modelId: ModelIdBox.Text,
            sequenceLength: (uint)SeqLenSlider.Value,
            batchSize: 1,
            quantization: "fp16"
        );

        ResultText.Text = $"Total VRAM: {prediction.TotalMb} MB";
    }
}
```

### Distribution: MSIX + WinGet

Use **Velopack** for auto-update:

```bash
cargo build --release --target x86_64-pc-windows-msvc
velopack pack --packId hwledger --packVersion 1.0.0 \
  --packDir apps/windows/bin/Release \
  --packAuthors "hwLedger" \
  --packTitle "hwLedger"
```

Publish to WinGet:

```bash
winget submit --token <github-token> hwledger-1.0.0.msix
```

## 3. Linux: cxx-qt + Qt 6 / QML

**cxx-qt** (maintained by KDAB) bridges Rust and Qt seamlessly. QML declarative UI, Rust business logic.

### Cargo.toml

```toml
[dependencies]
cxx-qt = { version = "0.7", features = ["qt6"] }
qt-build-utils = "0.7"

[build-dependencies]
qt-build-utils = "0.7"
```

### Rust Module (QObject)

`crates/hwledger-qt/src/planner.rs`:

```rust
use cxx_qt::CxxQtType;
use cxx_qt::prelude::*;

#[cxx_qt::bridge(crate = "hwledger_qt")]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt/planner.hpp");
        type Planner = crate::Planner;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[derive(Default)]
        pub struct Planner {
            #[qt_property(QString)]
            model_id: String,

            #[qt_property(u32)]
            sequence_length: u32,

            #[qt_property(u64)]
            estimated_vram_mb: u64,
        }
    }

    unsafe extern "RustQt" {
        #[slot]
        pub fn estimate_vram(self: Pin<&mut Planner>) {
            let core_planner = hwledger_core::Planner::new();
            
            let estimate = core_planner.estimate(hwledger_core::EstimateRequest {
                model_id: self.model_id.clone(),
                seq_len: self.sequence_length as usize,
                batch_size: 1,
                quantization: "fp16".to_string(),
            });

            self.estimated_vram_mb = estimate.total() / (1024 * 1024);
        }

        #[signal]
        pub fn vram_updated(self: Pin<&mut Planner>, vram_mb: u64);
    }
}
```

### QML UI

`apps/linux/qml/PlannerPage.qml`:

```qml
import QtQuick
import QtQuick.Controls
import HwledgerQt

Window {
    width: 800
    height: 600
    visible: true
    title: "hwLedger Planner"

    Planner {
        id: planner
        onVramUpdated: (vram_mb) => {
            resultText.text = `Total VRAM: ${vram_mb} MB`
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 10

        TextField {
            id: modelInput
            placeholderText: "Model ID"
            onTextChanged: planner.modelId = text
        }

        Slider {
            id: seqLenSlider
            from: 128
            to: 32768
            value: 4096
            onValueChanged: planner.sequenceLength = value
        }

        Button {
            text: "Estimate VRAM"
            onClicked: planner.estimateVram()
        }

        Text {
            id: resultText
            text: "Enter model and click Estimate"
        }
    }
}
```

## 4. Escape Hatch: Slint

If Qt integration proves too painful, **Slint** provides a native UI toolkit in Rust with JavaScript/TypeScript bindings.

```toml
[dependencies]
slint = { version = "1.8", features = ["backend-qt"] }
```

Slint UI (`.slint` file):

```
import { Button, LineEdit, Slider } from "std-widgets.slint";

export component Planner {
    in property <string> model_id <=> model-input.text;
    in property <float> sequence_length: 4096;
    out property <string> result_text;

    model-input := LineEdit {
        placeholder_text: "Model ID";
    }

    seq-slider := Slider {
        minimum: 128;
        maximum: 32768;
        value: sequence_length;
        changed => { root.sequence_length = self.value; }
    }

    Button {
        text: "Estimate VRAM";
        clicked => { root.invoke_estimate(); }
    }

    Text {
        text: result_text;
    }
}
```

Rust backend (single-threaded event loop):

```rust
slint::include_modules!();

fn main() {
    let ui = App::new();

    let ui_handle = ui.as_weak();
    ui.on_invoke_estimate(move || {
        let handle = ui_handle.clone();
        let result = estimate_vram(handle.borrow().get_model_id().to_string());
        handle.borrow_mut().set_result_text(result.into());
    });

    ui.run();
}

fn estimate_vram(model_id: String) -> String {
    let planner = hwledger_core::Planner::new();
    let estimate = planner.estimate(hwledger_core::EstimateRequest {
        model_id,
        seq_len: 4096,
        batch_size: 1,
        quantization: "fp16".to_string(),
    });
    format!("Total VRAM: {} MB", estimate.total() / (1024 * 1024))
}
```

## Recommendation Matrix

| Goal | Tool | Rationale |
|------|------|-----------|
| Ship MVP with native macOS | UniFFI | Proven, async-native, 1Password uses it |
| Windows .NET 9 native | csbindgen | AOT-friendly, WinGet distribution |
| Linux desktop | cxx-qt | Qt 6 is industry standard; LGPL compatible |
| Rapid prototyping | Slint | Single codebase, Rust-native, no C++ |
| Cross-platform (v2) | Tauri | Electron alternative; Rust backend |

## See also

- ADR-0001: Rust Core + Three Native GUIs
- ADR-0007: FFI Raw C Over UniFFI (decision history)
- `crates/hwledger-ffi/`
- `apps/macos/` (SwiftUI)
- `apps/windows/` (WinUI 3)
- `apps/linux/` (Qt 6)

## Sources

- [UniFFI Documentation](https://mozilla.github.io/uniffi-rs/)
- [cbindgen — C Header Generator](https://github.com/mozilla/cbindgen)
- [csbindgen — C# Binding Generator](https://github.com/mozilla/csbindgen)
- [cxx-qt Documentation](https://kdab.github.io/cxx-qt/)
- [Slint UI Framework](https://slint.dev/)
