import type { Component } from "solid-js";

/**
 * Fleet screen — STUB.
 *
 * Wiring to `hwledger-fleet-proto` over mTLS is tracked as a follow-up; the
 * SwiftUI app's Fleet screen uses an SSH-tunnelled gRPC channel (see
 * `apps/macos/HwLedger/Fleet/`). The Tauri equivalent will add a
 * `fleet_connect`/`fleet_list_hosts` tauri::command pair and reuse the same
 * wire types. Until then the screen is placeholder — it still ships for nav
 * parity with SwiftUI.
 */
export const FleetScreen: Component = () => (
  <>
    <header class="screen-header">
      <div>
        <h2>Fleet</h2>
        <p class="screen-hint">Manage remote hosts via mTLS (SwiftUI parity)</p>
      </div>
    </header>
    <section class="card">
      <p class="muted">
        Fleet wiring is deferred. The Rust side (<code>hwledger-fleet-proto</code>) already
        speaks gRPC-over-mTLS; the Tauri bridge will land behind
        <code> fleet_*</code> commands in a follow-up work package.
      </p>
    </section>
  </>
);
