pub mod bench_config;
pub mod cli;
pub mod speedtest;
pub mod ui;

#[cfg(feature = "branding")]
#[path = "branding/mod.rs"]
#[rustfmt::skip]
pub mod branding;
