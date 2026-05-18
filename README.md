# DMA Speedtest Memflow RS

Windows GUI for benchmarking DMA read performance through the [memflow](https://github.com/memflow/memflow) framework. Targets `explorer.exe`/`ntdll.dll` and reports throughput, read rate, and latency in real time.


## Metrics

Per chunk size and operation:

| Metric | Unit |
|--------|------|
| Throughput | MiB/s |
| Operation rate | ops/s |
| Latency (mean) | µs |


## CLI

| Flag | Default | Description |
|------|---------|-------------|
| `--connector` | `pcileech` | `pcileech` or `native` |
| `--device` | `FPGA` | PCILeech device string |
| `--duration` | `10` | Seconds per chunk size (1–60) |
| `--mode` | `read` | `read`, `write`, or `both` |
| `--sizes` | 4096, 8192, 16384, 32768 | Chunk sizes in bytes (comma-separated) |
| `-h`, `--help` | — | Usage and options |
| `-V`, `--version` | — | Package version |

## Requirements

- Windows 10/11 64-bit
- Rust toolchain for local builds
- PCILeech-compatible hardware and drivers for physical testing

## Credits

Built with [Rust](https://www.rust-lang.org/), [memflow](https://github.com/memflow/memflow), [tokio](https://github.com/tokio-rs/tokio) and [egui](https://github.com/emilk/).

License: **AGPL-3.0** (see `Cargo.toml`).
