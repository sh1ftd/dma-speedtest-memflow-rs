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

    pub fn len(self) -> u64 {
        self.end.saturating_sub(self.start)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafeWriteRegion {
    pub base: Address,
    pub size: umem,
}

pub struct ResolvedWriteTarget {
    pub base: Address,
    pub region_bytes: umem,
    pub verified_bytes: usize,
    pub restore_bytes: Vec<u8>,
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

fn subtract_range(segment: VaRange, excluded: VaRange) -> Vec<VaRange> {
    if !segment.overlaps(excluded) {
        return vec![segment];
    }

    let mut remaining = Vec::with_capacity(2);
    if segment.start < excluded.start {
        remaining.push(VaRange {
            start: segment.start,
            end: segment.end.min(excluded.start),
        });
    }
    if excluded.end < segment.end {
        remaining.push(VaRange {
            start: segment.start.max(excluded.end),
            end: segment.end,
        });
    }
    remaining
}

fn available_segments(region: VaRange, excluded: &[VaRange]) -> Vec<VaRange> {
    let mut segments = vec![region];
    for &excluded_range in excluded {
        segments = segments
            .into_iter()
            .flat_map(|segment| subtract_range(segment, excluded_range))
            .collect();
        if segments.is_empty() {
            break;
        }
    }
    segments
}

/// Pick the largest writable VA segment after carving out excluded ranges.
pub fn find_safe_write_region(
    map: &[MemoryRange],
    excluded: &[VaRange],
    min_bytes: usize,
) -> Option<SafeWriteRegion> {
    let min_bytes = min_bytes as u64;
    let mut best: Option<SafeWriteRegion> = None;

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
        for segment in available_segments(region, excluded) {
            let segment_size = segment.len();
            if segment_size < min_bytes {
                continue;
            }
            let candidate = SafeWriteRegion {
                base: Address::from(segment.start),
                size: segment_size as umem,
            };
            match best {
                None => best = Some(candidate),
                Some(best_region) if segment_size > best_region.size => best = Some(candidate),
                _ => {}
            }
        }
    }

    best
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

pub fn capture_write_restore_bytes(
    process: &mut IntoProcessInstanceArcBox<'_>,
    write_addr: Address,
    restore_bytes: usize,
) -> Result<Vec<u8>> {
    if restore_bytes == 0 {
        bail!("write restore capture requires a non-zero byte count");
    }

    let mut original = vec![0u8; restore_bytes];
    if mem_io::read_raw_into_with_retry(process, write_addr, &mut original) != IoAttempt::Ok {
        bail!(
            "read original write probe bytes ({restore_bytes} B) failed after {MAX_IO_RETRIES} retries; refusing write benchmark without restore data"
        );
    }

    Ok(original)
}

pub fn restore_write_target(
    process: &mut IntoProcessInstanceArcBox<'_>,
    write_addr: Address,
    original: &[u8],
) -> Result<()> {
    if original.is_empty() {
        return Ok(());
    }

    if mem_io::write_raw_with_retry(process, write_addr, original) != IoAttempt::Ok {
        bail!(
            "restore of original write probe bytes ({} B) failed after {MAX_IO_RETRIES} retries",
            original.len()
        );
    }

    Ok(())
}

/// Resolve a safe write base address inside `process` (same target as reads).
pub fn resolve_safe_write_target(
    process: &mut IntoProcessInstanceArcBox<'_>,
    read_addr: Address,
    min_bytes: usize,
) -> Result<ResolvedWriteTarget> {
    let min_bytes = min_bytes.max(MIN_WRITE_REGION_BYTES);

    let modules = collect_module_ranges(process)?;
    let mut excluded = modules;
    excluded.push(VaRange::from_start_size(
        read_addr,
        min_bytes.try_into().unwrap_or(u32::MAX as umem),
    ));

    let map = process.mapped_mem_range_vec(0, Address::null(), Address::invalid());
    let region = find_safe_write_region(&map, &excluded, min_bytes).ok_or_else(|| {
        anyhow::anyhow!(
            "no auto-selected writable probe region found (need at least {min_bytes} bytes outside loaded modules and the read probe page)"
        )
    })?;

    let verify_bytes = usize::try_from(region.size)
        .unwrap_or(usize::MAX)
        .min(min_bytes);
    let restore_bytes = capture_write_restore_bytes(process, region.base, verify_bytes)?;

    let verify_result = verify_write_target(process, region.base, verify_bytes);
    let restore_result = restore_write_target(process, region.base, &restore_bytes);

    match (verify_result, restore_result) {
        (Ok(()), Ok(())) => {}
        (Err(verify_err), Ok(())) => return Err(verify_err),
        (Ok(()), Err(restore_err)) => return Err(restore_err),
        (Err(verify_err), Err(restore_err)) => {
            bail!(
                "{verify_err}; additionally failed to restore original write probe bytes: {restore_err}"
            );
        }
    }

    Ok(ResolvedWriteTarget {
        base: region.base,
        region_bytes: region.size,
        verified_bytes: verify_bytes,
        restore_bytes,
    })
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
            end: 0x20000,
        }];
        let region = find_safe_write_region(&map, &excluded, 32 * 1024).unwrap();
        assert_eq!(region.base, Address::from(0x30000_u64));
        assert_eq!(region.size, 48 * 1024);
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
            Some(SafeWriteRegion {
                base: Address::from(0x2000_u64),
                size: 16 * 1024
            })
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
    fn excluded_range_blocks_candidate_when_remainder_is_too_small() {
        let map = vec![CTup3(
            Address::from(0x4000_u64),
            64 * 1024,
            PageType::WRITEABLE,
        )];
        let excluded = vec![VaRange {
            start: 0x4000,
            end: 0x5000,
        }];
        assert!(find_safe_write_region(&map, &excluded, 64 * 1024).is_none());
    }

    #[test]
    fn partial_excluded_range_is_carved_from_candidate() {
        let map = vec![CTup3(
            Address::from(0x4000_u64),
            64 * 1024,
            PageType::WRITEABLE,
        )];
        let excluded = vec![VaRange {
            start: 0x4000,
            end: 0x5000,
        }];
        let region = find_safe_write_region(&map, &excluded, 4096).unwrap();
        assert_eq!(region.base, Address::from(0x5000_u64));
        assert_eq!(region.size, 64 * 1024 - 0x1000);
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
