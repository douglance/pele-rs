#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
cargo fmt --all --check
cargo test --locked -p pele-rs --test conformance --test source_compat --test module_contract --test pie_compat --test packet_compat --test quadrant_compat --test radar_compat --test timeline_compat --test journey_compat --test flowchart_compat --test kanban_compat --test mindmap_compat --test class_compat --test er_compat --test xychart_compat --test state_compat --test gantt_compat --test sequence_compat --test git_graph_compat --test architecture_compat --test sankey_compat --test treemap_compat --test c4_compat --test info_compat --test requirement_compat --test block_compat --test railroad_compat --test cynefin_compat --test treeview_compat --test agentflow_compat --test wardley_compat --test eventmodeling_compat --test usecase_compat --test venn_compat --test ishikawa_compat --test swimlane_compat --test public_pie --test public_info --test public_sankey --test public_packet --test public_radar --test public_quadrant --test public_treemap --test public_journey --test public_xychart --test public_kanban --test public_mindmap --test public_timeline --test public_shared_functions --test public_flowchart --test public_swimlane --test public_er --test public_architecture --test public_git_graph --test public_agentflow --test public_ishikawa --test public_methods --test public_module_contract
cargo run --locked -p pele-rs --example corpus
cargo package --locked --allow-dirty -p pele-rs
cargo run --locked --manifest-path tests/rust-consumer/Cargo.toml
scripts/build-native.sh
cc -Wall -Wextra -Werror -I include tests/ffi.c target/release/libpele_ffi.a -o target/ffi-integration
target/ffi-integration
cc -Wall -Wextra -Werror -I include tests/ffi-public.c target/release/libpele_ffi.a -o target/ffi-public-integration
target/ffi-public-integration
if [[ "$(uname -s)" == Darwin ]]; then
    swift test --package-path bindings/swift
    swift run --package-path bindings/swift pele-smoke "$repo_dir" "$repo_dir/dist/evidence"
    if [[ -f Package.swift ]]; then swift run --package-path tests/swift-consumer; fi
    npm run test:parity
fi
scripts/build-wasm.sh
npm run test:web
git diff --check
