# Library architecture

The engine implements all 33 pinned Pele diagram families. The old standalone
flowchart renderer and its v1 JSON, C, Swift and web APIs have been removed.

The active contracts are [full-library architecture](FULL_PARITY_LLD.md),
[public models](LLD_PUBLIC_MODELS.md), [prepared entries](NATIVE_DIAGRAM_ENTRIES.md)
and [caller-authored model construction](DECLARED_MODEL_CONSTRUCTION.md).
Parsing, layout and SVG stay in Rust; host resources and presentation stay in
thin Swift and WebAssembly adapters. Registries and diagram module interfaces
support extension without changing the built-ins.

The optional [native graph scene API](NATIVE_GRAPH_SCENES.md) accepts directed graphs with caller identities and returns orthogonal geometry and SVG without changing the pinned Mermaid-family contracts.
