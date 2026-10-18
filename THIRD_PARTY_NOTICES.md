# Third-party notices

- **Pele**: Copyright (c) 2026 kepano. Complete upstream MIT license is retained
  verbatim in [third_party/PELE-LICENSE](third_party/PELE-LICENSE), including its
  additional Lucide ISC notice. No Lucide icon is used in this implementation.
- **Mermaid documentation fixtures**: Copyright (c) 2014 - 2022 Knut Sveidqvist.
  MIT, retained in [third_party/MERMAID-LICENSE](third_party/MERMAID-LICENSE).
  Pinned source provenance lives in `fixtures/pele/source.json`.
- **Noto Sans Regular**: Copyright 2018 The Noto Project Authors. Unmodified font
  is bundled in [crates/pele/assets/fonts](crates/pele/assets/fonts/README.md), with
  the full [SIL OFL 1.1](crates/pele/assets/fonts/OFL.txt). Font is not relicensed MIT.
- Rust and npm dependencies retain their respective upstream licenses. Their
  exact versions and sources are recorded in `Cargo.lock` and `package-lock.json`.
  Pele's npm package is a development-only compatibility oracle, not a runtime
  dependency or a renderer used by the playground.

The Rust source is MIT licensed under the root [LICENSE](LICENSE). See
[UPSTREAM.md](UPSTREAM.md) for the implementation relationship and limitations.
