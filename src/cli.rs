use crate::connector::Connector;
use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    #[arg(value_enum, short, long, default_value_t = Connector::Pcileech)]
    pub connector: Connector,

    /// PCILeech device name only used with pcileech connector
    #[arg(long, default_value = "FPGA")]
    pub pcileech_device: String,

    #[arg(short, long, default_value_t = 5)]
    pub duration: u64,
}
