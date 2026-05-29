use super::bench::BenchMode;
use super::connector::Connector;
use super::probe_targets::{TARGET_PROCESS, TARGET_READ_MODULE};
use super::write_target::{self, MIN_WRITE_REGION_BYTES};
use anyhow::Result;
use memflow::{plugins::Inventory, prelude::v1::*};

pub struct SpeedTestInit {
    pub process: IntoProcessInstanceArcBox<'static>,
    pub read_addr: Address,
    pub write_addr: Option<Address>,
    pub write_region_bytes: Option<umem>,
    pub write_verified_bytes: Option<usize>,
}

pub(super) fn initialize_speedtest(
    connector: Connector,
    pcileech_device: String,
    mode: BenchMode,
    max_chunk_bytes: usize,
) -> Result<SpeedTestInit> {
    let os = initialize_os(connector, &pcileech_device)?;
    let mut process = find_target_process(os)?;
    let read_addr = find_module_address(&mut process)?;

    let min_write_bytes = MIN_WRITE_REGION_BYTES.max(max_chunk_bytes);

    let (write_addr, write_region_bytes, write_verified_bytes) = if mode.needs_write_target() {
        let (addr, region, verified) =
            write_target::resolve_safe_write_target(&mut process, read_addr, min_write_bytes)?;
        (Some(addr), Some(region), Some(verified))
    } else {
        (None, None, None)
    };

    Ok(SpeedTestInit {
        process,
        read_addr,
        write_addr,
        write_region_bytes,
        write_verified_bytes,
    })
}

fn initialize_os(connector: Connector, pcileech_device: &str) -> Result<OsInstanceArcBox<'static>> {
    let mut inventory = Inventory::scan();

    match connector {
        Connector::Pcileech => initialize_pcileech(&mut inventory, pcileech_device),
        Connector::Native => Ok(memflow_native::create_os(
            &Default::default(), // os_cfg
            Default::default(),  // process_cfg
        )?),
        // Not tested
        Connector::Kvm | Connector::Qemu => initialize_vm_connector(&mut inventory, &connector),
    }
}

fn initialize_pcileech(
    inventory: &mut Inventory,
    device: &str,
) -> Result<OsInstanceArcBox<'static>> {
    let args = Args::new().insert("device", device);

    let connector_args = ConnectorArgs::new(None, args, None);

    inventory
        .builder()
        .connector("pcileech")
        .args(connector_args)
        .os("win32")
        .build()
        .map_err(|e| anyhow::anyhow!(
            "PCILeech connector error: {e}\n\nCommon fixes:\n  1. Ensure FPGA device is properly connected\n  2. Check if PCILeech driver is installed (FTDI)\n  3. Run as Administrator"
        ))
}

fn find_target_process(
    os: OsInstanceArcBox<'static>,
) -> Result<IntoProcessInstanceArcBox<'static>> {
    let process = os.into_process_by_name(TARGET_PROCESS)?;
    Ok(process)
}

fn find_module_address(process: &mut IntoProcessInstanceArcBox<'_>) -> Result<Address> {
    let addr = process.module_by_name(TARGET_READ_MODULE)?.base;
    Ok(addr)
}

// Not tested
fn initialize_vm_connector(
    inventory: &mut Inventory,
    connector: &Connector,
) -> Result<OsInstanceArcBox<'static>> {
    let args = Args::new()
        .insert("retries", "1")
        .insert("retry_interval", "0");
    let connector_args = ConnectorArgs::new(None, args, None);

    Ok(inventory
        .builder()
        .connector(&connector.to_string())
        .args(connector_args)
        .os("win32")
        .build()?)
}
