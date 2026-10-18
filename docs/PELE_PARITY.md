# Pinned Pele parity inventory

Target: [obsidianmd/pele 458322a5b8abb1ae57ddf127c32017ab9fb14a92](https://github.com/obsidianmd/pele/tree/458322a5b8abb1ae57ddf127c32017ab9fb14a92), package 1.6.0. Baseline: pele-rs `6803707`, inspected 2026-10-18. This baseline inventory defines required work. Subsequent bounded implementation evidence lives in [FULL_PARITY_PROGRESS.md](FULL_PARITY_PROGRESS.md); baseline markers below are not current completion claims.

`P` means existing partial implementation; `—` means absent from the full compatibility interface. Columns are syntax / semantic model / rendering / host. A `P` is not a passing full-parity gate. All rows are mandatory; no first-slice exclusion remains an exemption. Evidence must later record exact fixture IDs, counts, failures, skips and platforms against a source commit. See [FULL_PARITY_LLD.md](FULL_PARITY_LLD.md) for interfaces and acceptance rules.

Each row covers **every branch and public behavior in its listed source files**, including cases not summarized in the feature column. Full source/test obligations are listed below; summaries never narrow their scope.

## All 33 built-in families

| Upstream type | Model / source directory | Required semantic and rendering scope | Layout implementation | S/M/R/H at baseline |
| --- | --- | --- | --- | --- |
| `flowchart` | `FlowDb`; `src/diagrams/flowchart/` | Full lexical states and identifiers; all links/arrowheads/min lengths/edge IDs/expansions; shape metadata and aliases; nested/collapsed groups, directions, inheritance, classes/styles, click links, icons/images, animation, diagnostics and accessibility. | layered + compound + shared shapes/edges | P/P/P/P (legacy subset only) |
| `sequence` | `SeqDb`; `src/diagrams/sequence/` | Actors/participants/aliases/boxes; every message arrow; activations; notes; loops/alt/opt/par/critical/break/rect and nested blocks; create/destroy; numbering; links/properties; wrapping and mirrored actors. | sequence renderer, actors and colors | —/—/—/— |
| `class` | `ClassDb`; `src/diagrams/class/` | Members/methods/generics/visibility/static/abstract; aliases/annotations; all relation ends/cardinalities; namespaces; notes/interfaces; directions; styles/classes/click links. | class graph + layered/compound + markers/members | —/—/—/— |
| `state` | `StateDb`; `src/diagrams/state/` | Both headers; state descriptions; start/end/choice/fork/join; nested composites, parallel regions and local direction; transitions, notes, classes/styles and links. | state graph + layered/compound | —/—/—/— |
| `er` | `ErDb`; `src/diagrams/er/` | Entities/aliases/attributes/key/comment fields; all cardinalities and word aliases; identifying relationships; groups/local direction; classes/styles. | ER graph + layered/compound + markers | —/—/—/— |
| `pie` | `PieModel`; `src/diagrams/pie/` | Values/showData; titles/accessibility; duplicate/negative/zero/empty semantics; percentages, tiny slices, donutHole, textPosition, legends and responsive placement. | pie sector/label/legend renderer | —/—/—/— |
| `gantt` | `GanttDb`; `src/diagrams/gantt/` | Date formats, durations/dependencies, sections, exclusions/includes, task states, milestones/vertical markers, clicks, axes/ticks, todayMarker, displayMode and injected now. | gantt dates + axis + renderer | —/—/—/— |
| `gitGraph` | `GitModel`; `src/diagrams/git/` | Commit/branch/checkout/merge/cherry-pick population and errors; types/IDs/messages/tags; ordering; direction; parallelCommits and render configuration. | git AST/build model + branch/commit renderer | —/—/—/— |
| `mindmap` | `MindmapDb`; `src/diagrams/mindmap/` | Indentation/root rules; all delimiter shapes; icons/classes/markdown; branch sections/colors; balanced branches and narrow outline alternative. | mindmap renderer + common outline | —/—/—/— |
| `timeline` | `TimelineDb`; `src/diagrams/timeline/` | Titles/sections/time periods/events; duplicate-section/default-section behavior; directions; accessible labels; color settings and vertical/mobile adaptation. | timeline renderer | —/—/—/— |
| `journey` | `JourneyDb`; `src/diagrams/journey/` | Section/task/score/actor parsing; sorted actor semantics; score clamp/neutral handling; faces/card/actor legend; tags-as-text; vertical mobile layout. | journey renderer | —/—/—/— |
| `xychart` | `XyChartDb`; `src/diagrams/xychart/` | Band/numeric axes; bars/lines/data gaps; horizontal orientation; explicit/inferred ranges; clipping; ticks/labels/legends and xyChart config section. | xychart renderer + ticks | —/—/—/— |
| `quadrantChart` | `QuadrantDb`; `src/diagrams/quadrant/` | Quadrant/axis labels; coordinate rules; points; direct/class styles and precedence; labels/collision placement; markdown and layout configuration. | quadrant renderer | —/—/—/— |
| `requirement` | `RequirementDb`; `src/diagrams/requirement/` | Requirement and element kinds; IDs/text/risk/verification fields; every relationship and direction; missing endpoints; classes/styles/markdown; all relationship markers. | requirement graph + layered + markers | —/—/—/— |
| `c4` | `C4Db`; `src/diagrams/c4/` | All five C4 headers; all element macros/attributes; nested boundaries; relations/directions/tags/sprites; UpdateLayoutConfig/UpdateElementStyle/UpdateRelStyle and title/accessibility. | C4 layout + route + renderer | —/—/—/— |
| `sankey` | `SankeyDb`; `src/diagrams/sankey/` | CSV/quoted field preparation; duplicate nodes/links and numeric semantics; cyclic/invalid graphs; nodeAlignment/linkColor/value labels; dimension and node styling config. | Sankey flow layout | —/—/—/— |
| `block` | `BlockDb`; `src/diagrams/block/` | Column counts; nested composites; spanning blocks; spaces; all node/arrow shapes; links/edge styles/classes; independent child columns and grid sizing. | block grid layout + arrows | —/—/—/— |
| `packet` | `PacketModel`; `src/diagrams/packet/` | Bit ranges/auto placement; invalid spans; split fields/rows; numbering and descending order; row limits; configuration; narrow-row adaptation and label fitting. | packet model + field renderer | —/—/—/— |
| `kanban` | `KanbanDb`; `src/diagrams/kanban/` | Indentation columns/cards; YAML metadata; labels/assignee/ticket/priority; ticketBaseUrl policy; icons/classes/markdown; wrapped cards and folded board. | kanban renderer | —/—/—/— |
| `architecture` | `ArchitectureDb`; `src/diagrams/architecture/` | Services/groups/junctions/edges; side ports and arrows; alignment directives; icons/title/accessibility; group containment and duplicate/conflict placement. | architecture grid layout + icons | —/—/—/— |
| `radar` | `RadarModel`; `src/diagrams/radar/` | Axes/curves/named values/options; min/max/ticks/graticule; missing values; curve tension; legends/title; degenerate cases and responsive layout. | radar polar renderer | —/—/—/— |
| `treemap` | `TreemapModel`; `src/diagrams/treemap/` | Indent hierarchy/values; classes/styles; positive-value filtering; formatting specifiers/fallback; nested section headers and value-proportional layout. | treemap hierarchy + squarified layout + format | —/—/—/— |
| `info` | `InfoModel`; `src/diagrams/info/` | Keyword/showInfo and rejection rules; diagram/frontmatter title precedence; accessible names; pinned upstream version shown in compatibility mode. | info renderer | —/—/—/— |
| `agentflow` | `AgentflowDb`; `src/diagrams/agentflow/` | Tool/action/input/refdoc/decision/connector/task semantics; reference/failure/sequence edges; nested flows; attachments; metadata/classes/icons; source mappings and diagnostic rules. | agent graph/semantic normalization + shapes/colors + compound | —/—/—/— |
| `usecase` | `UsecaseModel`; `src/diagrams/usecase/` | Actors including hollow/awesome/icon/business; use-case forms/stereotypes; system/package nesting; include/extend/generalization/association arrows; notes/JSON nodes; classes/styles/animations/links. | usecase lexer/parser/resolution/JSON + graph + compound | —/—/—/— |
| `railroad` | `RailroadDb`; `src/diagrams/railroad/` | Constructor, EBNF, ABNF and PEG grammars; terminals/nonterminals; choices/options/repeats/counts/predicates/exceptions; rule links; accessible titles; narrow sequence folding. | four parsers + railroad IR + renderer | —/—/—/— |
| `treeView` | `TreeViewModel`; `src/diagrams/treeview/` | Indented and box-drawing syntax; original-line diagnostics; file/directory hierarchy; descriptions/highlights/classes; configured/resolved icons and extension mapping; narrow row adaptation. | tree model + boxDrawing + icons + renderer | —/—/—/— |
| `venn` | `VennDb`; `src/diagrams/venn/` | Set/subset sizes/labels/styles; pair/triple/higher intersections; duplicate/degenerate/numeric semantics; containment/touch/disjoint cases; label areas and layout config. | Venn geometric layout | —/—/—/— |
| `cynefin` | `CynefinModel`; `src/diagrams/cynefin/` | Domains/items/transitions; labels/styles/positions/config; model-building defaults; domain boundaries and transitions; title/accessibility. | Cynefin boundary geometry + renderer | —/—/—/— |
| `wardley` | `WardleyModel`; `src/diagrams/wardley/` | Components/links/evolution/notes; custom stages/labels/widths; sourcing/inertia; pipelines; annotations/accelerators/deaccelerators; percentage model and wardley-beta config. | Wardley parser/builder + map renderer | —/—/—/— |
| `swimlane` | `FlowchartModel (returned parsed model type remains flowchart upstream)`; `src/diagrams/swimlane/` | Entire flowchart grammar/model; top-level lanes, unnamed/default and nested/empty lanes; aligned ranks; ignored local lane direction; lane header rendering; adaptive turning. | shared flowchart + laneLayout | —/—/—/— |
| `ishikawa` | `IshikawaDb`; `src/diagrams/ishikawa/` | Both header aliases; effect/categories/indented causes/subcauses; alternating bones; empty/deep outline behavior; labels/wrapping and narrow tree layout. | Ishikawa outline/fishbone renderer | —/—/—/— |
| `eventmodeling` | `EventModelingModel`; `src/diagrams/eventmodeling/` | UI/processor/command/readmodel/event frames; namespaces/lanes; inferred/explicit links/reset frames; inline/data blocks retaining comments; notes/specifications and narrow layout. | event parser/model inference + lane renderer | —/—/—/— |

The names are exact discriminator names, not directory names: `gitGraph`, `quadrantChart`, `treeView`, and `xychart` must retain their capitalization. Detection contains 34 ordered entries because flowchart has two entries, but the registry has 33 families. Railroad's four grammars count as one family and four required parser variants. C4's five keywords count as one family and five required entry forms. Swimlane's parser returns FlowchartModel upstream; preserve that public model quirk in the upstream-shaped projection while keeping the enclosing document/render type `swimlane`.

## Public library and host matrix

| ID | Requirement / source | Baseline | Completion witness |
| --- | --- | --- | --- |
| API-01 | Full/core entry distinction; all exact type names; `register` replacement and `registered`; per-type exports. `core.ts`, `index.ts`, registry and all `entries/*.ts` | Absent | Every known type loaded/registered; empty core rejects before registration; replacement does not alter existing parsed document handler |
| API-02 | `detectType`: preprocessing + ordered detection, prefix/case quirks; `supports`: registered detector, not parse validity, catch preprocess errors | Absent | All 34 detector entries plus precedence and malformed-frontmatter public calls |
| API-03 | `parse(source, {limit,maxEdges,config})`: concrete family model with parse provenance and resolved config | Flowchart-only, incompatible shape/options | Entire family corpus normalized model equivalence; no invented generic node-only model |
| API-04 | `render(text OR parse-produced model, options)`: reuse without parse; fabricated model rejection; parsed handler/config retained | Source-only | Text/model outputs equal; options ignored at parse stage for model; repeated render and registry replacement |
| API-05 | Result `type,svg,width,height,links`; same link IDs decoding/internal flags; compatible errors | Missing type/links; v1 geometry extra | All types and link fixtures; typed models/results through Rust/C/Swift/WASM |
| API-06 | All render options from `types.ts`: measurer, fontFamily, fontFamilyMono, fontSize, idPrefix, responsive, maxWidth, autoDirection, directionBreakpoint, padding, limit, outputLimit, maxEdges, linkSchemes, imageSchemes, links, images, linkRel, icons, config, now | Different small option set | Per-option default/override/invalid case across every relevant consumer; no silent ignored mapped option |
| API-07 | `PeleError`: unsupported-diagram/syntax/semantic/limit, type/line/column/snippet/message; upstream code-unit locations | Different codes/spans | Rejected corpus comparison, multibyte/astral locations, no source logging |
| API-08 | `load`, `renderAsync`, `mountAsync`: deduplication, retry, null/unknown, detection failure, update-type prerequisites | Only whole WASM load | Concurrent/retry lifecycle integration, success/error matching synchronous interface |
| API-09 | `TextMeasurer`, `IconResolver`, custom Diagram registration and exports | No public resource seams | Real browser and native resource adapters, custom/replacement module lifecycle |
| API-10 | `forgetTextWidths`, bounded caches and loaded-font invalidation | Not applicable to legacy outlined text | Font loading changes layout; repeat render does not preserve fallback widths |
| HOST-01 | `mount` source/result/error/zoom state and update/refresh/destroy; last-good rendering/error overlay | Basic demo manually replaces SVG | Port every `tests/mount.test.ts` scenario through actual browser DOM |
| HOST-02 | Width observers, hidden/disconnected/moved-window behavior, font sets, resize-loop prevention, callback ordering, sanitizer | Absent | Real resize/hide/show/document move/font load; callback throws; retained drawing |
| HOST-03 | `enableZoom`, scale/fit, zoomBy/zoomTo/reset/refresh/destroy; pointer/pinch/wheel/keyboard/focus; controls/localization | Absent | Every zoom behavior with event cleanup; fits/clamps; multiple diagrams |
| HOST-04 | Native document/controller resources, accessible text, hit targets/links, viewport and zoom equivalents | Swift source render + raster smoke only | Standalone native view, all families render, lifecycle/error/recovery and native interactions |
| HOST-05 | Full v2 C ownership and Swift/WASM parity while retaining v1 consumers | v1 result ownership works | External consumers; parse/render lifetimes and concurrency; same complete corpus |
| HOST-06 | Full/minimal/lazy/per-type packaging and browser compatibility | Single WASM package | Actual package import paths and exported names; no built-in diagram implementation in JS |

## Shared implementation matrix

| ID | Required behavior / authoritative modules | Baseline difference | Gate |
| --- | --- | --- | --- |
| PRE-01 | CR/CRLF, HTML attribute quote normalization, frontmatter fences/indent/title/displayMode, comment handling/lineOffset. `preprocess.ts` | Frontmatter rejected; small comment scanner | Exact cleaned and comment-preserving source, config/title and public diagnostics |
| PRE-02 | Custom JSON-schema-like YAML reader: block/flow collections/scalars, quoted/plain/multiline, duplicate keys, depth 64, NaN/Infinity, JS key order. `util/yaml.ts` | Absent | Every YAML fixture fed through public source preparation/parse; no general YAML parser deviations |
| PRE-03 | Init/initialize/wrap directives; ignored malformed JSON; unsafe-key filtering; document deep merge and host shallow precedence | Directives rejected | Conflicting nested host/document sections, invalid/mixed directives |
| PRE-04 | Entity placeholder encoding/decoding; style/classDef color semicolon handling; exact location mapping | Plain literal labels | All named/numeric/invalid encodings and style-vs-label interactions |
| TEXT-01 | Label spans, markdown emphasis/dedent, inline HTML stripping/styles, line breaks, trimming, wrapping, long-word cases. `text/label.ts` and family-specific helpers | Literal labels and explicit breaks only | All rich labels, styles and narrow layouts; no source markup execution |
| TEXT-02 | Regular/bold Arial metrics, mono widths, Unicode wide/zero-width behavior, Canvas/custom measurement, fonts/cache. `text/{measurer,metrics}.ts` | Noto outlines, unsupported glyph/script errors | Same fixed measurer output as oracle; host-font change lifecycle; no false universal-font claim |
| TEXT-03 | Entity labels, icons in runs, missing-icon whitespace, per-render cache and recursive resolver. `text/{entities,icons}.ts` | No icons/entities | Present/missing/nested host resolver behavior, decoded resolver names |
| SVG-01 | Escaping, numeric formatting, label spans/weights, root classes/style/responsiveness and idPrefix. `svg/{builder,root}.ts` | Minimal standalone path SVG | Canonical DOM and options match; multiple mounted IDs safe |
| SVG-02 | All theme variables/series/critical/font/radius/weights; CSS style allowlists and class cleaning. `svg/theme.ts` | Five concrete colors | Light/dark/live CSS changes, every series slot and unsafe style fixture |
| SVG-03 | Full shape catalogue/aliases, dimensions, insets/spans, label geometry. `svg/shapes.ts`, `flowchart/shapes.ts` | Five shapes | Every canonical/alias/bracket/internal shape visual gallery and geometry witnesses |
| SVG-04 | Edge path styles, markers, trimming, looping, arrows, group title crossing. `svg/{edges,cluster,title}.ts` | Custom subset routes | All marker families, proper endpoints, title/path treatment and no omissions |
| SVG-05 | Source links/asset schemes/flags/rel and decoded IDs. `util/url.ts` plus renderers | Links/images rejected | Every approved/rejected URL including obfuscations, relative/internal links and custom schemes |
| SVG-06 | Trusted host icons versus source markup, built-in icons, remote images with host loader | No assets | Proper icon fallback/size/position and zero fetch on denied URL; trusted seam documented |
| SVG-07 | Accessible title/description/roles, model text, keyboard links, theme-safe focus | One root title only | Per-family accessibility corpus and host navigation |
| LAY-01 | Bounded layered algorithm: cycles/ranks/dummies/labels/crossing/Brandes–Köpf/settle/snap/tracks/ports. `layout/layered.ts` | Different simple router | Fixed-resource canonical edge/shape geometry and all layered public render cases |
| LAY-02 | Compound nested groups/ports/local direction/collapsed graph normalization. `layout/compound.ts` + family graph modules | Groups rejected | All nested cases, crossing/title cases, complete parent membership |
| LAY-03 | Swimlane rank alignment/lane membership/headers/unnamed groups. `layout/lanes.ts` | Absent | Lane-specific entire corpus in all directions/narrow modes |
| LAY-04 | Fit width/tighten/turn-to-fit/responsive/outline alternatives. `diagrams/common/{fit,fit-width,fit-text,outline}.ts` and renderers | No host width reflow | All max-width/mobile-layout cases preserve full model and own font size |
| LAY-05 | Specialized architecture/block/C4/Sankey/treemap/Venn and family-local render layouts | Absent | Every associated geometry/render/stress obligation |
| SAFE-01 | Default configurable UTF-16 source/output limits and maxEdges; host precedence | Different byte/fixed graph caps | Boundary and expanded-edge tests; same accepted budget corpus |
| SAFE-02 | All family work caps, finite output, nested input, pathological tokens/numbers; native panic and WASM trap recovery | Only subset caps | All stress cases and fixed-seed mutation runs, explicit measured resource budgets |
| SAFE-03 | SVG inertness policy across full feature set, URL/style/prototype/control characters, callback trust | Strict small element set | All render security corpora; no blanket bypass to add anchors/images/styles |
| DATA-01 | Complete typed per-family model, ordered maps, undefined/null distinction, parse-produced provenance | Immutable simple Flowchart only | Normalized entire model including diagnostic metadata |
| DATA-02 | Full scene/geometry v2, legacy v1 stays intact; native resolved export and browser compatible SVG | Geometry v1 graph only | Complete output on new entry and exact retained old fixture responses |
| OPS-01 | Upstream/fixture/font/icon licenses, pinned source-map inventory, external packages, complete evidence report | v0.1 notices/packages only | Distribution review; every source and required gate accounted for |

## Exact source ownership and fixture inventory

Paths below are relative to the pinned upstream checkout. File names are exhaustive for each family directory. Test columns list every matching test source directory/file plus corpus count; dynamic and mutated cases must be counted by the future extractor, not inferred from these seed counts. `tests/compat/<family>/SKIPPED.md`, where present, explains upstream-vs-Mermaid scope; it is not automatic permission to skip Pele behavior.

| Type | All upstream implementation files | Corpus seeds | Other required test sources |
| --- | --- | ---: | --- |
| `flowchart` | `db.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `shapes.ts`, `tokens.ts`, `types.ts` | 114 | `tests/flowchart/**`, `tests/compat/flowchart/**`, `tests/render/flowchart.test.ts` |
| `sequence` | `actors.ts`, `colors.ts`, `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 38 | `tests/sequence/**`, `tests/compat/sequence/**`, `tests/render/sequence-security.test.ts`, `tests/render/sequence.test.ts` |
| `class` | `db.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `markers.ts`, `members.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 38 | `tests/class/**`, `tests/compat/class/**`, `tests/render/class-security.test.ts`, `tests/render/class.test.ts` |
| `state` | `db.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 22 | `tests/state/**`, `tests/compat/state/**`, `tests/render/state-security.test.ts`, `tests/render/state.test.ts` |
| `er` | `db.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `markers.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 25 | `tests/er/**`, `tests/compat/er/**`, `tests/render/er-security.test.ts`, `tests/render/er.test.ts` |
| `pie` | `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 2 | `tests/pie/**`, `tests/compat/pie/**`, `tests/render/pie.test.ts` |
| `gantt` | `axis.ts`, `dates.ts`, `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 11 | `tests/gantt/**`, `tests/compat/gantt/**`, `tests/render/gantt-security.test.ts`, `tests/render/gantt.test.ts` |
| `gitGraph` | `db.ts`, `index.ts`, `parser.ts`, `render.ts` | 35 | `tests/git/**`, `tests/compat/git/**`, `tests/render/git-security.test.ts`, `tests/render/git.test.ts` |
| `mindmap` | `db.ts`, `index.ts`, `parser.ts`, `render.ts` | 13 | `tests/mindmap/**`, `tests/compat/mindmap/**`, `tests/render/mindmap-security.test.ts`, `tests/render/mindmap.test.ts` |
| `timeline` | `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts` | 14 | `tests/timeline/**`, `tests/compat/timeline/**`, `tests/render/timeline-security.test.ts`, `tests/render/timeline.test.ts` |
| `journey` | `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts` | 1 | `tests/journey/**`, `tests/compat/journey/**`, `tests/render/journey-security.test.ts`, `tests/render/journey.test.ts` |
| `xychart` | `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `ticks.ts`, `tokens.ts` | 8 | `tests/xychart/**`, `tests/compat/xychart/**`, `tests/render/xychart-security.test.ts`, `tests/render/xychart.test.ts` |
| `quadrantChart` | `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts` | 3 | `tests/quadrant/**`, `tests/compat/quadrant/**`, `tests/render/quadrant-security.test.ts`, `tests/render/quadrant.test.ts` |
| `requirement` | `db.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 9 | `tests/requirement/**`, `tests/compat/requirement/**`, `tests/render/requirement-security.test.ts`, `tests/render/requirement.test.ts` |
| `c4` | `db.ts`, `index.ts`, `layout.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `route.ts`, `tokens.ts`, `types.ts` | 6 | `tests/c4/**`, `tests/compat/c4/**`, `tests/render/c4-security.test.ts`, `tests/render/c4.test.ts` |
| `sankey` | `db.ts`, `index.ts`, `layout.ts`, `lexer.ts`, `parser.ts`, `render.ts` | 8 | `tests/sankey/**`, `tests/compat/sankey/**`, `tests/render/sankey-security.test.ts`, `tests/render/sankey.test.ts` |
| `block` | `arrows.ts`, `db.ts`, `index.ts`, `layout.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 30 | `tests/block/**`, `tests/compat/block/**`, `tests/render/block-security.test.ts`, `tests/render/block.test.ts` |
| `packet` | `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 4 | `tests/packet/**`, `tests/compat/packet/**`, `tests/render/packet-security.test.ts`, `tests/render/packet.test.ts` |
| `kanban` | `db.ts`, `index.ts`, `parser.ts`, `render.ts` | 3 | `tests/kanban/**`, `tests/compat/kanban/**`, `tests/render/kanban-security.test.ts`, `tests/render/kanban.test.ts` |
| `architecture` | `db.ts`, `icons.ts`, `index.ts`, `layout.ts`, `parser.ts`, `render.ts` | 6 | `tests/architecture/**`, `tests/compat/architecture/**`, `tests/render/architecture-security.test.ts`, `tests/render/architecture.test.ts` |
| `radar` | `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 3 | `tests/radar/**`, `tests/compat/radar/**`, `tests/render/radar-security.test.ts`, `tests/render/radar.test.ts` |
| `treemap` | `format.ts`, `index.ts`, `layout.ts`, `model.ts`, `parser.ts`, `render.ts` | 8 | `tests/treemap/**`, `tests/compat/treemap/**`, `tests/render/treemap-security.test.ts`, `tests/render/treemap.test.ts` |
| `info` | `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 2 | `tests/info/**`, `tests/compat/info/**`, `tests/render/info-security.test.ts`, `tests/render/info.test.ts` |
| `agentflow` | `colorSlots.ts`, `db.ts`, `diagnostics.ts`, `graph.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `shapes.ts`, `tokens.ts`, `types.ts` | 12 | `tests/agentflow/**`, `tests/compat/agentflow/**`, `tests/render/agentflow-security.test.ts`, `tests/render/agentflow.test.ts`, `tests/corpus/agentflow-reference.json` |
| `usecase` | `db.ts`, `graph.ts`, `index.ts`, `json.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `tokens.ts`, `types.ts` | 24 | `tests/usecase/**`, `tests/compat/usecase/**`, `tests/compat/usecase-parser/**`, `tests/render/usecase-security.test.ts`, `tests/render/usecase.test.ts` |
| `railroad` | `abnf.ts`, `common.ts`, `db.ts`, `ebnf.ts`, `index.ts`, `ir.ts`, `peg.ts`, `render.ts`, `types.ts` | 14 | `tests/railroad/**`, `tests/compat/railroad/**`, `tests/render/railroad-security.test.ts`, `tests/render/railroad.test.ts` |
| `treeView` | `boxDrawing.ts`, `icons.ts`, `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 13 | `tests/treeview/**`, `tests/compat/treeview/**`, `tests/render/treeview-security.test.ts`, `tests/render/treeview.test.ts` |
| `venn` | `db.ts`, `index.ts`, `layout.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `types.ts` | 8 | `tests/venn/**`, `tests/compat/venn/**`, `tests/render/venn-security.test.ts`, `tests/render/venn.test.ts` |
| `cynefin` | `boundaries.ts`, `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 3 | `tests/cynefin/**`, `tests/compat/cynefin/**`, `tests/render/cynefin-security.test.ts`, `tests/render/cynefin.test.ts` |
| `wardley` | `builder.ts`, `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 19 | `tests/wardley/**`, `tests/compat/wardley/**`, `tests/render/wardley-security.test.ts`, `tests/render/wardley.test.ts` |
| `swimlane` | `index.ts` | 13 | `tests/swimlane/**`, `tests/compat/swimlane/**`, `tests/render/swimlane.test.ts` |
| `ishikawa` | `db.ts`, `index.ts`, `lexer.ts`, `parser.ts`, `render.ts`, `types.ts` | 1 | `tests/ishikawa/**`, `tests/compat/ishikawa/**`, `tests/render/ishikawa-security.test.ts`, `tests/render/ishikawa.test.ts` |
| `eventmodeling` | `index.ts`, `model.ts`, `parser.ts`, `render.ts` | 12 | `tests/eventmodeling/**`, `tests/compat/eventmodeling/**`, `tests/render/eventmodeling-security.test.ts`, `tests/render/eventmodeling.test.ts` |

### Shared source modules (all required)

- `src/core.ts`
- `src/detect.ts`
- `src/diagrams/common/accDescr.ts`
- `src/diagrams/common/caseless.ts`
- `src/diagrams/common/fit-text.ts`
- `src/diagrams/common/fit-width.ts`
- `src/diagrams/common/fit.ts`
- `src/diagrams/common/generics.ts`
- `src/diagrams/common/icon.ts`
- `src/diagrams/common/literal-label.ts`
- `src/diagrams/common/outline.ts`
- `src/diagrams/common/scan.ts`
- `src/diagrams/common/tokens.ts`
- `src/diagrams/registry.ts`
- `src/errors.ts`
- `src/index.ts`
- `src/layout/compound.ts`
- `src/layout/lanes.ts`
- `src/layout/layered.ts`
- `src/lazy.ts`
- `src/models.ts`
- `src/mount.ts`
- `src/preprocess.ts`
- `src/svg/builder.ts`
- `src/svg/cluster.ts`
- `src/svg/edges.ts`
- `src/svg/root.ts`
- `src/svg/shapes.ts`
- `src/svg/theme.ts`
- `src/svg/title.ts`
- `src/text/entities.ts`
- `src/text/icons.ts`
- `src/text/label.ts`
- `src/text/measurer.ts`
- `src/text/metrics.ts`
- `src/types.ts`
- `src/util/chars.ts`
- `src/util/unicode.ts`
- `src/util/url.ts`
- `src/util/yaml.ts`
- `src/window.ts`
- `src/zoom.ts`

### Per-family public entry modules (all required)

`src/entries/agentflow.ts`, `src/entries/architecture.ts`, `src/entries/block.ts`, `src/entries/c4.ts`, `src/entries/class.ts`, `src/entries/cynefin.ts`, `src/entries/er.ts`, `src/entries/eventmodeling.ts`, `src/entries/flowchart.ts`, `src/entries/gantt.ts`, `src/entries/gitGraph.ts`, `src/entries/info.ts`, `src/entries/ishikawa.ts`, `src/entries/journey.ts`, `src/entries/kanban.ts`, `src/entries/mindmap.ts`, `src/entries/packet.ts`, `src/entries/pie.ts`, `src/entries/quadrantChart.ts`, `src/entries/radar.ts`, `src/entries/railroad.ts`, `src/entries/requirement.ts`, `src/entries/sankey.ts`, `src/entries/sequence.ts`, `src/entries/state.ts`, `src/entries/swimlane.ts`, `src/entries/timeline.ts`, `src/entries/treeView.ts`, `src/entries/treemap.ts`, `src/entries/usecase.ts`, `src/entries/venn.ts`, `src/entries/wardley.ts`, `src/entries/xychart.ts`.

Source coverage check: 212 family files + 42 shared files + 33 entry files = 287 TypeScript source files. All 287 files under pinned `src/` are accounted for. These counts inventory work; they do not measure implementation completion.

### Cross-family and host test sources

- `tests/browsers.test.ts`
- `tests/entries.test.ts`
- `tests/mount.test.ts`
- `tests/stress.test.ts`
- `tests/zoom.test.ts`
- `tests/util/measurer.test.ts`
- `tests/util/preprocess.test.ts`
- `tests/util/url.test.ts`
- `tests/util/yaml.test.ts`
- `tests/layout/compound.test.ts`
- `tests/layout/lanes.test.ts`
- `tests/website/completions.test.ts`
- `tests/website/examples.test.ts`
- `tests/render/all-types.test.ts`
- `tests/render/edges.test.ts`
- `tests/render/fonts.test.ts`
- `tests/render/inert.test.ts`
- `tests/render/max-width.test.ts`
- `tests/render/mobile-layouts.test.ts`
- `tests/render/security.test.ts`

All `tests/support/**` adapters/loaders, `tests/compat/**` inputs and grammar/reference dependencies are extraction/provenance inputs. Their private implementation assertions are re-expressed through public Rust/adapter interfaces; do not add private Rust unit tests. Website application UI itself is not the library target, but every documented diagram/example and public API behavior demonstrated there is included. The upstream build/registry-generation files govern package entry completeness.

## Complete shape catalogue obligation

From `src/diagrams/flowchart/shapes.ts`; every group below includes its aliases. These are not all interchangeable rectangles: sizing, drawing and attachment geometry come from `svg/shapes.ts`. Internal and bracket-syntax names also remain required where upstream produces them.

- `rect`, `proc`, `process`, `rectangle`, `squareRect`
- `rounded`, `event`, `roundedRect`
- `stadium`, `terminal`, `pill`
- `fr-rect`, `subprocess`, `subproc`, `framed-rectangle`, `subroutine`
- `cyl`, `db`, `database`, `cylinder`
- `datastore`, `data-store`
- `folder`, `directory`
- `bucket`
- `console`
- `browser`
- `person`
- `circle`, `circ`
- `bang`
- `cloud`
- `diam`, `decision`, `diamond`, `question`
- `hex`, `hexagon`, `prepare`
- `lean-r`, `lean-right`, `in-out`, `lean_right`
- `lean-l`, `lean-left`, `out-in`, `lean_left`
- `trap-b`, `priority`, `trapezoid-bottom`, `trapezoid`
- `trap-t`, `manual`, `trapezoid-top`, `inv-trapezoid`, `inv_trapezoid`
- `dbl-circ`, `double-circle`, `doublecircle`
- `text`
- `notch-rect`, `card`, `notched-rectangle`
- `lin-rect`, `lined-rectangle`, `lined-process`, `lin-proc`, `shaded-process`
- `sm-circ`, `start`, `small-circle`, `stateStart`
- `fr-circ`, `stop`, `framed-circle`, `stateEnd`
- `fork`, `join`, `forkJoin`
- `hourglass`, `collate`
- `brace`, `comment`, `brace-l`
- `brace-r`
- `braces`
- `bolt`, `com-link`, `lightning-bolt`
- `doc`, `document`
- `delay`, `half-rounded-rectangle`
- `h-cyl`, `das`, `horizontal-cylinder`
- `lin-cyl`, `disk`, `lined-cylinder`
- `curv-trap`, `curved-trapezoid`, `display`
- `div-rect`, `div-proc`, `divided-rectangle`, `divided-process`
- `tri`, `extract`, `triangle`
- `win-pane`, `internal-storage`, `window-pane`
- `f-circ`, `junction`, `filled-circle`
- `notch-pent`, `loop-limit`, `notched-pentagon`
- `flip-tri`, `manual-file`, `flipped-triangle`
- `sl-rect`, `manual-input`, `sloped-rectangle`
- `docs`, `documents`, `st-doc`, `stacked-document`
- `st-rect`, `procs`, `processes`, `stacked-rectangle`
- `bow-rect`, `stored-data`, `bow-tie-rectangle`
- `cross-circ`, `summary`, `crossed-circle`
- `tag-doc`, `tagged-document`
- `tag-rect`, `tagged-rectangle`, `tag-proc`, `tagged-process`
- `flag`, `paper-tape`
- `odd`, `rect_left_inv_arrow`
- `lin-doc`, `lined-document`

Internal names: `state`, `choice`, `note`, `composite`, `rectWithTitle`, `labelRect`, `block_arrow`, `collapsedGroup`, `iconSquare`, `iconCircle`, `icon`, `iconRounded`, `imageSquare`, `anchor`, `kanbanItem`, `mindmapCircle`, `defaultMindmapNode`, `classBox`, `erBox`, `requirementBox`, `usecaseActor`, `usecaseActorHollow`, `usecaseActorAwesome`, `usecaseActorIcon`, `usecaseBusiness`, `usecaseEllipse`, `usecaseJsonTable`.
Bracket mapping: `square` => `rect`, `round` => `rounded`, `ellipse` => `ellipse`, `hexagon` => `hex`, `diamond` => `diam`.

## Completion report contract

The future machine-readable manifest must give each case a stable ID and record: upstream SHA/path/test name; source/options/resources/clock; public operation; expected accept/reject; model normalization revision; render mode; required platforms; result classification and evidence path. Each inventory row records total/attempted/pass/fail/skip counts independently for syntax, semantics, rendering and host. Missing extraction, unsupported constructs, unavailable platforms and altered resource budgets remain visible. No row may be marked complete with skipped required cases or unexplained normalization exclusions.

Current full-parity completion: **none**. Flowchart has a working restricted v0.1 implementation; its full upstream row is incomplete. Existing release evidence stays valid for its explicitly documented scope. This inventory is deliberately exhaustive in obligation and conservative in completion status.
