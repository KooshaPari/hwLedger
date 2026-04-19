import SwiftUI

struct LedgerScreen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Ledger")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Timeline of dispatches, costs, audit events (event-sourced, hash-chain verifiable).")
                .font(.body)
                .foregroundColor(.secondary)
                .lineLimit(nil)

            Spacer()
        }
        .padding()
    }
}

#Preview {
    LedgerScreen()
}
