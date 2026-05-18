//! Safe writable target discovery for DMA write benchmarks.
//!
//! Writes never use module images or the read probe page; the address is not user-configurable.

use super::mem_io::{self, IoAttempt, MAX_IO_RETRIES};
use anyhow::{Result, bail};
use memflow::prelude::v1::*;

/// Minimum writable region size when resolving a write target (32 KiB).
pub const MIN_WRITE_REGION_BYTES: usize = 32 * 1024;

/// Extra bytes past each module end treated as non-writable.
const MODULE_END_GUARD_BYTES: u64 = 4096;

/// Inclusive-exclusive virtual range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaRange {
    pub start: u64,
    pub end: u64,
}

impl VaRange {
    pub fn from_start_size(start: Address, size: umem) -> Self {
        let start = start.to_umem();
        let end = start.saturating_add(size);
        Self { start, end }
    }

    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    pub fn with_guard_after_module(base: Address, size: umem) -> Self {
        let start = base.to_umem();
        let end = start
            .saturating_add(size)
            .saturating_add(MODULE_END_GUARD_BYTES);
        Self { start, end }
    }
}

pub fn collect_module_ranges(process: &mut IntoProcessInstanceArcBox<'_>) -> Result<Vec<VaRange>> {
    let modules = process.module_list()?;
    Ok(modules
        .into_iter()
        .map(|info| VaRange::with_guard_after_module(info.base, info.size))
        .collect())
}

fn is_writable_candidate(page_type: PageType) -> bool {
    page_type.contains(PageType::WRITEABLE) && !page_type.contains(PageType::PAGE_TABLE)
}

fn region_overlaps_excluded(region: VaRange, excluded: &[VaRange]) -> bool {
    excluded.iter().any(|&ex| region.overlaps(ex))
}

/// Pick the largest writable VA region that does not overlap excluded ranges.
pub fn find_safe_write_region(
    map: &[MemoryRange],
    excluded: &[VaRange],
    min_bytes: usize,
) -> Option<Address> {
    let min_bytes = min_bytes as u64;
    let mut best: Option<(u64, Address)> = None;

    for range in map {
        let base = range.0;
        let size = range.1;
        let page_type = range.2;
        if !is_writable_candidate(page_type) {
            continue;
        }
        let size_u = size;
        if size_u < min_bytes {
            continue;
        }
        let region = VaRange::from_start_size(base, size);
        if region_overlaps_excluded(region, excluded) {
            continue;
        }
        match best {
            None => best = Some((size_u, base)),
            Some((best_size, _)) if size_u > best_size => best = Some((size_u, base)),
            _ => {}
        }
    }

    best.map(|(_, addr)| addr)
}

fn fill_verify_pattern(buf: &mut [u8]) {
    for (i, b) in buf.iter_mut().enumerate() {
        *b = 0xA5_u8.wrapping_add((i % 256) as u8);
    }
}

/// Write/read-back `verify_bytes` at `write_addr` (full intended chunk footprint).
pub fn verify_write_target(
    process: &mut IntoProcessInstanceArcBox<'_>,
    write_addr: Address,
    verify_bytes: usize,
) -> Result<()> {
    if verify_bytes == 0 {
        bail!("write verification requires a non-zero byte count");
    }

    let mut canary = vec![0u8; verify_bytes];
    fill_verify_pattern(&mut canary);

    if mem_io::write_raw_with_retry(process, write_addr, &canary) != IoAttempt::Ok {
        bail!(
            "write canary ({verify_bytes} B) to candidate region failed after {MAX_IO_RETRIES} retries (partial virtual write)"
        );
    }

    let mut read_back = vec![0u8; verify_bytes];
    if mem_io::read_raw_into_with_retry(process, write_addr, &mut read_back) != IoAttempt::Ok {
        bail!(
            "read back canary ({verify_bytes} B) from candidate region failed after {MAX_IO_RETRIES} retries (partial virtual read)"
        );
    }

    if read_back != canary {
        bail!(
            "write verification failed at {write_addr} ({verify_bytes} B): DMA write path returned mismatched data"
        );
    }

    Ok(())
}

/// Resolve a safe write base address inside `process` (same target as reads).
pub fn resolve_safe_write_target(
    process: &mut IntoProcessInstanceArcBox<'_>,
    read_addr: Address,
    min_bytes: usize,
) -> Result<(Address, umem)> {
    let min_bytes = min_bytes.max(MIN_WRITE_REGION_BYTES);

    let modules = collect_module_ranges(process)?;
    let mut excluded = modules;
    excluded.push(VaRange::from_start_size(
        read_addr,
        min_bytes.try_into().unwrap_or(u32::MAX as umem),
    ));

    let map = process.mapped_mem_range_vec(0, Address::null(), Address::invalid());
    let write_addr = find_safe_write_region(&map, &excluded, min_bytes).ok_or_else(|| {
        anyhow::anyhow!(
            "no safe writable region found (need at least {min_bytes} bytes outside loaded modules and the read probe page)"
        )
    })?;

    let region_size = map
        .iter()
        .find(|range| range.0 == write_addr)
        .map(|range| range.1)
        .unwrap_or(min_bytes as umem);

    let verify_bytes = (region_size as usize).min(min_bytes);
    verify_write_target(process, write_addr, verify_bytes)?;

    Ok((write_addr, region_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range_fully_inside(outer: VaRange, inner: VaRange) -> bool {
        inner.start >= outer.start && inner.end <= outer.end
    }

    #[test]
    fn ranges_overlap_when_shared_bytes() {
        let a = VaRange { start: 0, end: 100 };
        let b = VaRange {
            start: 50,
            end: 150,
        };
        assert!(a.overlaps(b));
    }

    #[test]
    fn range_fully_inside_detects_containment() {
        let outer = VaRange {
            start: 0,
            end: 1000,
        };
        let inner = VaRange {
            start: 100,
            end: 200,
        };
        assert!(range_fully_inside(outer, inner));
        assert!(!range_fully_inside(inner, outer));
    }

    #[test]
    fn ranges_do_not_overlap_when_adjacent() {
        let a = VaRange { start: 0, end: 100 };
        let b = VaRange {
            start: 100,
            end: 200,
        };
        assert!(!a.overlaps(b));
    }

    #[test]
    fn find_largest_non_overlapping_region() {
        let map = vec![
            CTup3(Address::from(0x1000_u64), 16 * 1024, PageType::WRITEABLE),
            CTup3(Address::from(0x10000_u64), 64 * 1024, PageType::WRITEABLE),
            CTup3(Address::from(0x30000_u64), 48 * 1024, PageType::WRITEABLE),
        ];
        let excluded = vec![VaRange {
            start: 0x10000,
            end: 0x10008,
        }];
        let addr = find_safe_write_region(&map, &excluded, 32 * 1024).unwrap();
        assert_eq!(addr, Address::from(0x30000_u64));
    }

    #[test]
    fn page_table_regions_are_skipped() {
        let map = vec![CTup3(
            Address::from(0x5000_u64),
            64 * 1024,
            PageType::WRITEABLE | PageType::PAGE_TABLE,
        )];
        assert!(find_safe_write_region(&map, &[], 4096).is_none());
    }

    #[test]
    fn regions_below_min_bytes_are_skipped() {
        let map = vec![CTup3(
            Address::from(0x2000_u64),
            16 * 1024,
            PageType::WRITEABLE,
        )];
        assert!(find_safe_write_region(&map, &[], 32 * 1024).is_none());
        assert_eq!(
            find_safe_write_region(&map, &[], 16 * 1024),
            Some(Address::from(0x2000_u64))
        );
    }

    #[test]
    fn non_writable_regions_are_skipped() {
        let map = vec![CTup3(
            Address::from(0x8000_u64),
            64 * 1024,
            PageType::READ_ONLY,
        )];
        assert!(find_safe_write_region(&map, &[], 4096).is_none());
    }

    #[test]
    fn excluded_range_blocks_overlapping_candidate() {
        let map = vec![CTup3(
            Address::from(0x4000_u64),
            64 * 1024,
            PageType::WRITEABLE,
        )];
        let excluded = vec![VaRange {
            start: 0x4000,
            end: 0x5000,
        }];
        assert!(find_safe_write_region(&map, &excluded, 4096).is_none());
    }

    #[test]
    fn module_guard_extends_past_module_end() {
        let guard = VaRange::with_guard_after_module(Address::from(0x1000_u64), 0x1000);
        assert_eq!(guard.start, 0x1000);
        assert_eq!(guard.end, 0x1000 + 0x1000 + MODULE_END_GUARD_BYTES);
    }

    #[test]
    fn verify_pattern_is_deterministic_and_wraps() {
        let mut buf = [0u8; 300];
        fill_verify_pattern(&mut buf);
        assert_eq!(buf[0], 0xA5);
        assert_eq!(buf[255], 0xA5_u8.wrapping_add(255));
        assert_eq!(buf[256], 0xA5);
    }
}
