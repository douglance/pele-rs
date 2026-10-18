#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/env.sh"
cargo fmt --all --check
cargo test --locked --release -p pele-rs --test conformance --test source_compat --test module_contract --test pie_compat --test packet_compat --test quadrant_compat --test radar_compat --test timeline_compat --test journey_compat --test flowchart_compat --test kanban_compat --test mindmap_compat --test class_compat --test er_compat --test xychart_compat --test state_compat --test gantt_compat --test sequence_compat --test git_graph_compat --test architecture_compat --test sankey_compat --test treemap_compat --test c4_compat --test info_compat --test requirement_compat --test block_compat --test railroad_compat --test cynefin_compat --test treeview_compat --test agentflow_compat --test wardley_compat --test eventmodeling_compat --test usecase_compat --test venn_compat --test ishikawa_compat --test swimlane_compat --test public_pie --test public_info --test public_sankey --test public_packet --test public_radar --test public_quadrant --test public_treemap --test public_journey --test public_xychart --test public_kanban --test public_mindmap --test public_timeline --test public_shared_functions --test public_flowchart --test public_swimlane --test public_er --test public_architecture --test public_git_graph --test public_agentflow --test public_ishikawa --test public_class --test public_eventmodeling --test public_state --test public_gantt --test public_sequence --test public_venn --test public_treeview --test public_c4 --test public_requirement --test public_usecase --test public_cynefin --test public_railroad --test public_block --test public_wardley --test public_error_values --test public_binary --test public_render_operation --test public_methods --test public_module_contract --test public_source_limit --test public_output_limit --test public_edge_limit --test public_registry
cargo run --locked -p pele-rs --example corpus
cargo package --locked --allow-dirty -p pele-rs
cargo run --locked --manifest-path tests/rust-consumer/Cargo.toml
scripts/build-native.sh
cc -Wall -Wextra -Werror -I include tests/ffi.c target/release/libpele_ffi.a -o target/ffi-integration
target/ffi-integration
cc -Wall -Wextra -Werror -I include tests/ffi-public.c target/release/libpele_ffi.a -o target/ffi-public-integration
target/ffi-public-integration
cc -Wall -Wextra -Werror -std=c11 -I include tests/ffi-source-limit.c target/release/libpele_ffi.a -o target/ffi-source-limit-integration
target/ffi-source-limit-integration
cc -Wall -Wextra -Werror -std=c11 -I include tests/ffi-output-limit.c target/release/libpele_ffi.a -o target/ffi-output-limit-integration
target/ffi-output-limit-integration
cc -Wall -Wextra -Werror -std=c11 -I include tests/ffi-parse-limits.c target/release/libpele_ffi.a -o target/ffi-parse-limits-integration
target/ffi-parse-limits-integration
cc -Wall -Wextra -Werror -std=c11 -I include tests/ffi-registry.c target/release/libpele_ffi.a -o target/ffi-registry-integration
target/ffi-registry-integration
if [[ "$(uname -s)" == Darwin ]]; then
    PELE_VERIFY_MUTABLE=1 swift test -c release --package-path bindings/swift
    swift run -c release --package-path bindings/swift pele-smoke "$repo_dir" "$repo_dir/dist/evidence"
    if [[ -f Package.swift ]]; then swift run -c release --package-path tests/swift-consumer; fi
    npm run test:parity
fi
scripts/build-wasm.sh
npm run test:web
git diff --check
