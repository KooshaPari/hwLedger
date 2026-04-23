# Broken mermaid fixture

This fixture exercises the `mermaid` check with a deliberately malformed
diagram — the gate must produce at least one Error finding for it.

```mermaid
unknownKind TD
  A[open --> B
  C -->
```
