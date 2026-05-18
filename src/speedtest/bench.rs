//! Benchmark operation and mode types shared by CLI and GUI.

/// Which memory operations to run during a session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BenchMode {
    #[default]
    Read,
    Write,
    Both,
}

impl BenchMode {
    pub fn needs_write_target(self) -> bool {
        matches!(self, BenchMode::Write | BenchMode::Both)
    }

    pub fn ops_for_size(self) -> &'static [BenchOp] {
        match self {
            BenchMode::Read => &[BenchOp::Read],
            BenchMode::Write => &[BenchOp::Write],
            BenchMode::Both => &[BenchOp::Read, BenchOp::Write],
        }
    }
}

/// Single benchmark operation (read or write).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BenchOp {
    Read,
    Write,
}

impl BenchOp {
    pub fn label(self) -> &'static str {
        match self {
            BenchOp::Read => "read",
            BenchOp::Write => "write",
        }
    }

    pub fn ops_per_sec_label(self) -> &'static str {
        match self {
            BenchOp::Read => "reads/s",
            BenchOp::Write => "writes/s",
        }
    }
}

/// Live stats: `(op, throughput_mib_s, ops_per_sec, elapsed_secs, chunk_size, latency_us)`.
pub type BenchStats = (BenchOp, f64, u64, f64, usize, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_mode_does_not_need_write_target() {
        assert!(!BenchMode::Read.needs_write_target());
        assert_eq!(BenchMode::Read.ops_for_size(), &[BenchOp::Read]);
    }

    #[test]
    fn write_and_both_need_write_target() {
        assert!(BenchMode::Write.needs_write_target());
        assert!(BenchMode::Both.needs_write_target());
    }

    #[test]
    fn both_mode_runs_read_then_write_per_size() {
        assert_eq!(
            BenchMode::Both.ops_for_size(),
            &[BenchOp::Read, BenchOp::Write]
        );
    }

    #[test]
    fn bench_op_labels_for_cli_and_gui() {
        assert_eq!(BenchOp::Read.label(), "read");
        assert_eq!(BenchOp::Write.label(), "write");
        assert_eq!(BenchOp::Read.ops_per_sec_label(), "reads/s");
        assert_eq!(BenchOp::Write.ops_per_sec_label(), "writes/s");
    }
}
