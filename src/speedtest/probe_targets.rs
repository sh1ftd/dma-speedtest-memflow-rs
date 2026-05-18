//! Human-readable descriptions of fixed read/write probe targets (not user-configurable).

use crate::bench_config::format_byte_count;
use memflow::prelude::v1::*;

pub const TARGET_PROCESS: &str = "explorer.exe";
pub const TARGET_READ_MODULE: &str = "ntdll.dll";
pub const WRITE_PAYLOAD_DESC: &str = "rotating bytes (buffer[i] = i % 251)";
pub const WRITE_CANARY_BYTES: usize = 64;

#[derive(Clone, Copy, Debug)]
pub struct ProbeTargets {
    pub read_addr: Address,
    pub write_addr: Option<Address>,
    pub write_region_bytes: Option<umem>,
}

impl ProbeTargets {
    pub fn new(
        read_addr: Address,
        write_addr: Option<Address>,
        write_region_bytes: Option<umem>,
    ) -> Self {
        Self {
            read_addr,
            write_addr,
            write_region_bytes,
        }
    }

    pub fn format_va(addr: Address) -> String {
        format!("{:#x}", addr.to_umem())
    }

    pub fn write_region_end(&self) -> Option<Address> {
        match (self.write_addr, self.write_region_bytes) {
            (Some(base), Some(size)) => Some(Address::from(base.to_umem().saturating_add(size))),
            _ => None,
        }
    }

    /// Lines logged once at connect (CLI console + GUI console).
    pub fn connect_detail_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("Process: {TARGET_PROCESS}"),
            format!(
                "Read probe: {TARGET_READ_MODULE} @ {} (module image base; read-only benchmark)",
                Self::format_va(self.read_addr)
            ),
        ];

        if let (Some(addr), Some(region)) = (self.write_addr, self.write_region_bytes) {
            let end = self
                .write_region_end()
                .map(Self::format_va)
                .unwrap_or_else(|| "?".to_string());
            lines.push(format!(
                "Write probe: auto-selected private writable region @ {}",
                Self::format_va(addr)
            ));
            lines.push(format!(
                "  Region size: {}  (VA [{}, {}))",
                format_byte_count(region as usize),
                Self::format_va(addr),
                end
            ));
            lines.push(format!("  Payload per op: {WRITE_PAYLOAD_DESC}"));
            lines.push(
                "  Excluded at selection: all PE modules (+4 KiB past each), read probe page"
                    .to_string(),
            );
            lines.push(format!(
                "  Verified at connect: {WRITE_CANARY_BYTES}-byte write-read canary"
            ));
        }

        lines
    }

    pub fn format_read_pass(&self, chunk_bytes: usize) -> String {
        format!(
            "DMA read {chunk} from {TARGET_READ_MODULE} @ {addr} (module base)",
            chunk = format_byte_count(chunk_bytes),
            addr = Self::format_va(self.read_addr),
        )
    }

    pub fn format_write_pass(&self, chunk_bytes: usize) -> Option<String> {
        let addr = self.write_addr?;
        let region = self.write_region_bytes?;
        Some(format!(
            "DMA write {chunk} -> {addr} (inside {region} writable region); {WRITE_PAYLOAD_DESC}",
            chunk = format_byte_count(chunk_bytes),
            addr = Self::format_va(addr),
            region = format_byte_count(region as usize),
        ))
    }

    /// Short line for live stats / repeated CLI context during a write pass.
    pub fn format_write_live(&self, chunk_bytes: usize) -> Option<String> {
        let addr = self.write_addr?;
        Some(format!(
            "write @ {} | chunk {} | {WRITE_PAYLOAD_DESC}",
            Self::format_va(addr),
            format_byte_count(chunk_bytes),
        ))
    }
}
