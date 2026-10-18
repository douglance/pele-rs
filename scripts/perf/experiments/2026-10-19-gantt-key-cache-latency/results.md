# Matched Gantt cache follow-up

Status: **complete**. Baseline commit 42abbfbead6a8fa0c23bd578b6f09b78c747056e. Candidate differs only by the supplied Gantt runtime patch.

This report measures native API returns at explicitly fixed histories. Final returned-value disposal, setup, snapshots/hashing and I/O are outside the operation timer. It is distinct from the historical allocation experiment and from end-to-end Swift API measurements. Seven paired observations do not establish statistical significance; no runtime adoption follows automatically.

Coverage: history128=large-only; 23 conditions; 161 recorded paired trials. Hard invocation ceiling 60s.
Omitted latency coverage: gantt/small history128 render-mutable-core; gantt/small history128 render-mutable-publish; gantt/medium history128 render-mutable-core; gantt/medium history128 render-mutable-publish. Coverage was fixed before execution; no midrun row selection.
Setup counters: 6008 prior-history renders; 66 source preparations; 24 mutation/recovery probes; 101 warmup resets. Warmup cap hits: 5.

| Case | History | Native API/control | Count | Baseline µs | Candidate µs | Paired Δµs |
|---|---:|---|---:|---:|---:|---:|
| gantt/small | 1 | render-mutable-core | 1 | 219.208 | 160.166 | -13.250 |
| gantt/small | 1 | render-mutable-publish | 1 | 204.500 | 212.625 | -0.333 |
| gantt/small | 16 | render-mutable-core | 1 | 163.750 | 150.042 | -14.583 |
| gantt/small | 16 | render-mutable-publish | 1 | 202.625 | 191.458 | -8.000 |
| gantt/medium | 1 | render-mutable-core | 1 | 765.792 | 706.000 | -48.083 |
| gantt/medium | 1 | render-mutable-publish | 1 | 864.750 | 779.000 | -59.750 |
| gantt/medium | 16 | render-mutable-core | 1 | 816.458 | 687.459 | -87.583 |
| gantt/medium | 16 | render-mutable-publish | 1 | 894.375 | 795.125 | -59.750 |
| gantt/large | 1 | render-mutable-core | 1 | 3104.875 | 2796.875 | -296.500 |
| gantt/large | 1 | render-mutable-publish | 1 | 3275.750 | 3066.250 | -160.125 |
| gantt/large | 16 | render-mutable-core | 1 | 3182.625 | 3033.000 | -227.249 |
| gantt/large | 16 | render-mutable-publish | 1 | 3517.000 | 3117.792 | -354.958 |
| gantt/large | 128 | render-mutable-core | 1 | 3303.000 | 3161.042 | -237.375 |
| gantt/large | 128 | render-mutable-publish | 1 | 5250.750 | 4554.417 | -721.793 |
| pie/small | 1 | render-mutable-core | 128 | 26.548 | 30.827 | 3.547 |
| pie/small | 1 | render-mutable-publish | 128 | 29.262 | 29.169 | 0.268 |
| gantt/small | 1 | property-unique | 128 | 23.115 | 23.243 | -1.973 |
| pie/medium | 1 | render-mutable-core | 128 | 156.134 | 157.478 | 3.937 |
| pie/medium | 1 | render-mutable-publish | 128 | 155.689 | 163.828 | 6.535 |
| gantt/medium | 1 | property-unique | 128 | 137.824 | 142.112 | 2.407 |
| pie/large | 1 | render-mutable-core | 64 | 241.934 | 228.730 | -8.326 |
| pie/large | 1 | render-mutable-publish | 64 | 253.173 | 246.286 | -5.988 |
| gantt/large | 1 | property-unique | 32 | 640.284 | 658.962 | 25.234 |

Negative paired delta means lower observed candidate latency in this run. Source-equivalent whole SVG, before/after graph identity/order/descriptors, revisions and publication snapshots are checked for every pair. Gantt histories reset for each single-operation sample. Pie and unique-key controls use matched calibrated batch counts. Raw observations and ordering remain in the adjacent JSON.

Provenance:

- Runtime patch SHA-256: 6fbc2b2402ef3954be7b27d576cb1b0815fe553a025f2c7a1cebf6f1fef29168.
- Workflow hashes: recorded in JSON.
- Pinned oracle SHA-256: d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d.
- baseline source input SHA-256: a931716f5cb79b6c7d386d9e20adce4a5816b934a88e079eb29573d92f7161cd; executable SHA-256: 717b9a56605d1997f3dc44b43b189c4adb584259da76c175c97739730921287d.
- candidate source input SHA-256: 2566807a331c7055838e0902de071042f1e20290130f30f5e36a686a1caf2125; executable SHA-256: 61f024290925ee8e05ee6fff52b8a39461ba6cda35d009255cf674fd2db89e02.

Shared-host process/load snapshots (no argument/environment dumps):

```json
{
  "before": {
    "loadAverage": [
      12.3173828125,
      14.2783203125,
      12.75048828125
    ],
    "freeMemory": 91291648,
    "processes": [
      {
        "pid": 37164,
        "cpuPercent": 97.6,
        "name": "xctest",
        "testScope": "external"
      },
      {
        "pid": 410,
        "cpuPercent": 43.4,
        "name": "WindowServer"
      },
      {
        "pid": 41390,
        "cpuPercent": 41.4,
        "name": "node"
      },
      {
        "pid": 32704,
        "cpuPercent": 38,
        "name": "WhatsApp"
      },
      {
        "pid": 40661,
        "cpuPercent": 33,
        "name": "Google Chrome Helper (Renderer)"
      },
      {
        "pid": 813,
        "cpuPercent": 18.4,
        "name": "Google Chrome Helper"
      },
      {
        "pid": 524,
        "cpuPercent": 12.5,
        "name": "appleh13camerad"
      }
    ]
  },
  "after": {
    "loadAverage": [
      10.177734375,
      13.51171875,
      12.541015625
    ],
    "freeMemory": 110788608,
    "processes": [
      {
        "pid": 37164,
        "cpuPercent": 96.1,
        "name": "xctest",
        "testScope": "external"
      },
      {
        "pid": 40661,
        "cpuPercent": 53.3,
        "name": "Google Chrome Helper (Renderer)"
      },
      {
        "pid": 410,
        "cpuPercent": 45.8,
        "name": "WindowServer"
      },
      {
        "pid": 32704,
        "cpuPercent": 43.3,
        "name": "WhatsApp"
      },
      {
        "pid": 41390,
        "cpuPercent": 31.7,
        "name": "node"
      },
      {
        "pid": 813,
        "cpuPercent": 20.1,
        "name": "Google Chrome Helper"
      },
      {
        "pid": 41457,
        "cpuPercent": 15.5,
        "name": "pele-perf"
      },
      {
        "pid": 41458,
        "cpuPercent": 12.4,
        "name": "pele-perf"
      },
      {
        "pid": 524,
        "cpuPercent": 12,
        "name": "appleh13camerad"
      },
      {
        "pid": 629,
        "cpuPercent": 10.5,
        "name": "Google Chrome"
      }
    ]
  }
}
```
