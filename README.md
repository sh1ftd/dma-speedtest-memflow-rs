# DMA Speedtest Memflow RS

A Windows command-line tool for benchmarking DMA (Direct Memory Access) read/write speeds using the memflow framework.

## Features

- Measure real-world DMA read/write performance and latency
- Comprehensive speed testing (sequential/random/bulk reads)
- Detailed performance metrics (throughput, latency, error rates)
- Support for multiple connector types

## Command Line Usage

```bash
dma-speedtest-memflow-rs [OPTIONS]

Options:
  -c, --connector <CONNECTOR>       [default: pcileech] [possible values: pcileech, native, qemu, kvm]
  --pcileech-device <DEVICE>        PCILeech device name [default: FPGA]
  -d, --duration <DURATION>         Test duration in seconds [default: 5]
  -h, --help                        Print help
```

### Connector Examples

```bash
# Physical DMA hardware
dma-speedtest-memflow-rs --connector pcileech

# Local testing
dma-speedtest-memflow-rs --connector native

# Virtual machine testing
dma-speedtest-memflow-rs --connector qemu

# KVM testing
dma-speedtest-memflow-rs --connector kvm
```

## Quick Start

```bash
# Clone and build
git clone https://github.com/sh1ftd/dma-speedtest-memflow-rs.git
cd dma-speedtest-memflow-rs
cargo build --release

# Run
./target/release/dma-speedtest-memflow-rs
```

## Requirements

- Windows OS (64-bit)
- Compatible DMA hardware (for pcileech connector)
- Administrator privileges
- Appropriate drivers for chosen connector

## Credits

Built with [Rust](https://www.rust-lang.org/) and [memflow](https://github.com/memflow/memflow)
