# Specifications

Acceptance criteria:
- Every referenced docs media asset exists and is non-empty.
- Every committed CLI, Streamlit, and referenced GUI journey has a
  `manifest.verified.json`.
- Each manifest step has a usable `screenshot_path`.
- Rich MP4 coverage is required for CLI, Streamlit, and referenced GUI journeys.
- Unreferenced GUI captures may warn when Remotion is sandbox-blocked, but they
  must not break referenced docs pages.
