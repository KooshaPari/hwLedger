import SwiftUI

struct LibraryScreen: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 24) {
            Text("Library")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Grid of models (local GGUF, MLX, Ollama, HF-pulled metadata). Search + filter by arch kind.")
                .font(.body)
                .foregroundColor(.secondary)
                .lineLimit(nil)

            Spacer()
        }
        .padding()
    }
}

#Preview {
    LibraryScreen()
}
