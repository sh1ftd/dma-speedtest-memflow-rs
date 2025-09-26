# DMA Speedtest Memflow RS

Windows GUI for benchmarking DMA read performance through the [memflow](https://github.com/memflow/memflow) framework. Targets `explorer.exe`/`ntdll.dll` and reports throughput, read rate, and latency in real time.

## Features

- PCILeech and memflow-native connectors
- Batch testing across configurable read sizes
- Live plots for throughput, reads/s, and latency with per-size summaries
- Embedded console, adjustable UI scale, and resizable plots

## Build from source

```bash
git clone https://github.com/sh1ftd/dma-speedtest-memflow-rs.git
cd dma-speedtest-memflow-rs
cargo build --release
./target/release/dma-speedtest-memflow-rs
```

## Usage

1. Launch the binary.
2. Pick a connector:
   - `PCILeech` (requires hardware device identifier)
   - `Native` (memflow-native for local testing)
3. Set test duration (1–60 s) and enable desired read sizes.
4. Start the test, observe results.

## UI Overview

- **Configuration:** connector selection, duration slider, read-size grid with bulk actions, UI scale input.
- **Results:** throughput/reads/latency plots, min/avg/max tables, live metrics banner, chunk progress, console toggle.
- **Console:** optional overlay listing size transitions, stats updates, and errors.

## Requirements

- Windows 10/11 64-bit
- Rust toolchain for local builds
- PCILeech-compatible hardware and drivers for physical testing

## Credits

Built with [Rust](https://www.rust-lang.org/), [memflow](https://github.com/memflow/memflow) and [egui](https://github.com/emilk/
egui).
