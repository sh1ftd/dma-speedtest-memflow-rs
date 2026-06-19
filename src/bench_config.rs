//! Shared benchmark chunk sizes and formatting (CLI + GUI).

use crate::speedtest::MIN_WRITE_REGION_BYTES;
use anyhow::{Result, bail};

/// Default enabled chunk sizes (4–32 KiB).
pub const DEFAULT_CHUNK_SIZES: [usize; 4] = [4096, 8192, 16384, 32768];

/// All chunk sizes offered in the GUI size grid.
pub const GUI_CHUNK_SIZES: [usize; 9] = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072];

/// Largest accepted benchmark chunk (16 MiB).
pub const MAX_CHUNK_SIZE_BYTES: usize = 16 * 1024 * 1024;

pub fn default_gui_chunk_sizes() -> Vec<(usize, bool)> {
    GUI_CHUNK_SIZES
        .into_iter()
        .map(|size| (size, DEFAULT_CHUNK_SIZES.contains(&size)))
        .collect()
}

/// Largest enabled GUI chunk, at least [`MIN_WRITE_REGION_BYTES`].
pub fn max_enabled_chunk_bytes(test_sizes: &[(usize, bool)]) -> usize {
    test_sizes
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(size, _)| *size)
        .max()
        .unwrap_or(MIN_WRITE_REGION_BYTES)
        .max(MIN_WRITE_REGION_BYTES)
}

/// Largest chunk in a list (CLI `--sizes`), at least [`MIN_WRITE_REGION_BYTES`].
pub fn max_chunk_bytes_in_list(sizes: &[usize]) -> usize {
    sizes
        .iter()
        .copied()
        .max()
        .unwrap_or(MIN_WRITE_REGION_BYTES)
        .max(MIN_WRITE_REGION_BYTES)
}

pub fn default_chunk_sizes_csv() -> String {
    DEFAULT_CHUNK_SIZES
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Human-readable chunk size (KiB for ≥1024 B).
pub fn format_chunk_size(size: usize) -> String {
    if size >= 1024 {
        format!("{} KiB", size / 1024)
    } else {
        format!("{size} B")
    }
}

/// Byte count with optional KiB suffix (e.g. probe region size).
pub fn format_byte_count(bytes: usize) -> String {
    if bytes >= 1024 {
        format!("{} KiB ({bytes} B)", bytes / 1024)
    } else {
        format!("{bytes} B")
    }
}

pub fn parse_chunk_sizes_csv(input: &str) -> Result<Vec<usize>> {
    let mut out = Vec::new();
    for part in input.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let n: usize = t.parse().map_err(|_| {
            anyhow::anyhow!(
                "invalid chunk size {t:?} (expected positive integers, comma-separated)"
            )
        })?;
        if n == 0 {
            bail!("chunk sizes must be positive; got 0");
        }
        if n > MAX_CHUNK_SIZE_BYTES {
            bail!("chunk size {n} B exceeds maximum allowed size of {MAX_CHUNK_SIZE_BYTES} B");
        }
        out.push(n);
    }
    validate_chunk_sizes(&out)?;
    Ok(out)
}

pub fn chunk_sizes_from_optional_csv(s: &str) -> Result<Option<Vec<usize>>> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_chunk_sizes_csv(trimmed)?))
}

pub fn validate_chunk_sizes(sizes: &[usize]) -> Result<()> {
    if sizes.is_empty() {
        bail!("chunk sizes must contain at least one value");
    }
    if sizes.contains(&0) {
        bail!("chunk sizes must be positive; got 0");
    }
    if let Some(max_size) = sizes
        .iter()
        .copied()
        .find(|&size| size > MAX_CHUNK_SIZE_BYTES)
    {
        bail!("chunk size {max_size} B exceeds maximum allowed size of {MAX_CHUNK_SIZE_BYTES} B");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_chunk_sizes_are_expected() {
        assert_eq!(DEFAULT_CHUNK_SIZES, [4096, 8192, 16384, 32768]);
    }

    #[test]
    fn format_chunk_size_formats_bytes_under_1k() {
        assert_eq!(format_chunk_size(0), "0 B");
        assert_eq!(format_chunk_size(512), "512 B");
    }

    #[test]
    fn format_chunk_size_formats_kib_from_1024() {
        assert_eq!(format_chunk_size(1024), "1 KiB");
        assert_eq!(format_chunk_size(4096), "4 KiB");
    }

    #[test]
    fn format_byte_count_includes_raw_bytes_for_kib() {
        assert_eq!(format_byte_count(4096), "4 KiB (4096 B)");
    }

    #[test]
    fn parse_chunk_sizes_csv_accepts_spaces() {
        assert_eq!(
            parse_chunk_sizes_csv("4096, 8192 ,16384").unwrap(),
            vec![4096, 8192, 16384]
        );
    }

    #[test]
    fn parse_chunk_sizes_csv_rejects_zero() {
        assert!(parse_chunk_sizes_csv("4096,0").is_err());
    }

    #[test]
    fn parse_chunk_sizes_csv_rejects_values_above_limit() {
        let too_large = MAX_CHUNK_SIZE_BYTES + 1;
        let err = parse_chunk_sizes_csv(&too_large.to_string()).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum allowed size"));
    }

    #[test]
    fn validate_chunk_sizes_rejects_direct_zero_values() {
        assert!(validate_chunk_sizes(&[4096, 0]).is_err());
    }

    #[test]
    fn validate_chunk_sizes_rejects_direct_values_above_limit() {
        assert!(validate_chunk_sizes(&[MAX_CHUNK_SIZE_BYTES + 1]).is_err());
    }
}
