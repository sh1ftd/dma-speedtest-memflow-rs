use super::{BenchMode, Connector, PassSummary, ProbeTargets};
use anyhow::{Result, bail};
use clap::ValueEnum;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    Csv,
    Json,
}

impl ReportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkReport {
    pub version: String,
    pub connector: String,
    pub mode: String,
    pub duration_secs: u64,
    pub sizes: Vec<usize>,
    pub generated_unix_secs: u64,
    pub probes: ReportProbeTargets,
    pub passes: Vec<PassSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportProbeTargets {
    pub read_addr: String,
    pub write_addr: Option<String>,
    pub write_region_bytes: Option<usize>,
}

impl BenchmarkReport {
    pub fn new(
        connector: Connector,
        mode: BenchMode,
        duration_secs: u64,
        sizes: &[usize],
        probes: ProbeTargets,
        passes: Vec<PassSummary>,
    ) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            connector: connector.to_string(),
            mode: bench_mode_label(mode).to_string(),
            duration_secs,
            sizes: sizes.to_vec(),
            generated_unix_secs: unix_timestamp_secs(),
            probes: ReportProbeTargets {
                read_addr: ProbeTargets::format_va(probes.read_addr),
                write_addr: probes.write_addr.map(ProbeTargets::format_va),
                write_region_bytes: probes
                    .write_region_bytes
                    .map(|bytes| usize::try_from(bytes).unwrap_or(usize::MAX)),
            },
            passes,
        }
    }
}

pub fn infer_report_format(path: &Path) -> Result<ReportFormat> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("csv") => Ok(ReportFormat::Csv),
        Some("json") => Ok(ReportFormat::Json),
        _ => bail!("could not infer report format from output path; use --output-format csv|json"),
    }
}

pub fn resolve_report_format(format: Option<ReportFormat>, path: &Path) -> Result<ReportFormat> {
    match format {
        Some(format) => Ok(format),
        None => infer_report_format(path),
    }
}

pub fn default_report_path(format: ReportFormat) -> PathBuf {
    PathBuf::from("reports").join(format!(
        "dma-speedtest-{}.{}",
        unix_timestamp_millis(),
        format.extension()
    ))
}

pub fn write_report_to_path(
    report: &BenchmarkReport,
    format: ReportFormat,
    path: &Path,
) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    let body = match format {
        ReportFormat::Csv => report_to_csv(report),
        ReportFormat::Json => serde_json::to_string_pretty(report)?,
    };
    fs::write(path, body)?;
    Ok(())
}

fn report_to_csv(report: &BenchmarkReport) -> String {
    let mut out = String::new();
    out.push_str(
        "version,connector,mode,duration_secs,generated_unix_secs,read_addr,write_addr,write_region_bytes,op,chunk_bytes,samples,total_ops,measured_secs,min_mib_s,avg_mib_s,max_mib_s,min_ops_s,avg_ops_s,max_ops_s,min_latency_us,avg_latency_us,max_latency_us\n",
    );

    for pass in &report.passes {
        let columns = [
            report.version.clone(),
            report.connector.clone(),
            report.mode.clone(),
            report.duration_secs.to_string(),
            report.generated_unix_secs.to_string(),
            report.probes.read_addr.clone(),
            report.probes.write_addr.clone().unwrap_or_default(),
            report
                .probes
                .write_region_bytes
                .map(|bytes| bytes.to_string())
                .unwrap_or_default(),
            pass.op.label().to_string(),
            pass.chunk_bytes.to_string(),
            pass.samples.to_string(),
            pass.total_ops.to_string(),
            format!("{:.6}", pass.measured_secs),
            format!("{:.6}", pass.min_mib_s),
            format!("{:.6}", pass.avg_mib_s),
            format!("{:.6}", pass.max_mib_s),
            format!("{:.6}", pass.min_ops_s),
            format!("{:.6}", pass.avg_ops_s),
            format!("{:.6}", pass.max_ops_s),
            format!("{:.6}", pass.min_latency_us),
            format!("{:.6}", pass.avg_latency_us),
            format!("{:.6}", pass.max_latency_us),
        ];
        out.push_str(
            &columns
                .into_iter()
                .map(csv_escape)
                .collect::<Vec<_>>()
                .join(","),
        );
        out.push('\n');
    }

    out
}

fn csv_escape(value: String) -> String {
    if value.contains(|c| [',', '"', '\n', '\r'].contains(&c)) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn bench_mode_label(mode: BenchMode) -> &'static str {
    match mode {
        BenchMode::Read => "read",
        BenchMode::Write => "write",
        BenchMode::Both => "both",
    }
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speedtest::BenchOp;
    use memflow::prelude::v1::*;

    fn summary() -> PassSummary {
        PassSummary {
            op: BenchOp::Read,
            chunk_bytes: 4096,
            min_mib_s: 10.0,
            avg_mib_s: 20.0,
            max_mib_s: 30.0,
            min_ops_s: 100.0,
            avg_ops_s: 200.0,
            max_ops_s: 300.0,
            min_latency_us: 1.0,
            avg_latency_us: 2.0,
            max_latency_us: 3.0,
            samples: 4,
            total_ops: 1000,
            measured_secs: 5.0,
        }
    }

    #[test]
    fn infers_report_format_from_extension() {
        assert_eq!(
            infer_report_format(Path::new("out.csv")).unwrap(),
            ReportFormat::Csv
        );
        assert_eq!(
            infer_report_format(Path::new("out.JSON")).unwrap(),
            ReportFormat::Json
        );
        assert!(infer_report_format(Path::new("out.txt")).is_err());
    }

    #[test]
    fn serializes_report_as_json_and_csv() {
        let report = BenchmarkReport::new(
            Connector::Native,
            BenchMode::Read,
            1,
            &[4096],
            ProbeTargets::new(Address::from(0x1000_u64), None, None),
            vec![summary()],
        );

        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"connector\":\"native\""));
        assert!(json.contains("\"avg_mib_s\":20.0"));

        let csv = report_to_csv(&report);
        assert!(csv.contains("version,connector,mode"));
        assert!(csv.contains(",native,read,"));
        assert!(csv.contains(",20.000000,"));
    }

    #[test]
    fn writes_report_to_nested_output_path() {
        let report = BenchmarkReport::new(
            Connector::Native,
            BenchMode::Read,
            1,
            &[4096],
            ProbeTargets::new(Address::from(0x1000_u64), None, None),
            vec![summary()],
        );
        let dir = std::env::temp_dir().join(format!(
            "dma-speedtest-report-test-{}",
            unix_timestamp_millis()
        ));
        let path = dir.join("nested").join("report.json");

        write_report_to_path(&report, ReportFormat::Json, &path).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("\"connector\": \"native\""));

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir.join("nested")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
