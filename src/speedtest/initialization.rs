use super::connector::Connector;
use anyhow::Result;
use memflow::{plugins::Inventory, prelude::v1::*};

const TARGET_PROCESS: &str = "explorer.exe";
const TARGET_MODULE: &str = "ntdll.dll";

pub(super) fn initialize_speedtest(
    connector: Connector,
    pcileech_device: String,
) -> Result<(IntoProcessInstanceArcBox<'static>, Address)> {
    let os = initialize_os(connector, &pcileech_device)?;
    let mut process = find_target_process(os)?;
    let test_addr = find_module_address(&mut process)?;

    Ok((process, test_addr))
}

fn initialize_os(connector: Connector, pcileech_device: &str) -> Result<OsInstanceArcBox<'static>> {
    let inventory = Inventory::scan();

    match connector {
        Connector::Pcileech => initialize_pcileech(&inventory, pcileech_device),
        Connector::Native => Ok(memflow_native::create_os(
            &Default::default(), // os_cfg
            Default::default(),  // process_cfg
        )?),
        // Not tested
        Connector::Kvm | Connector::Qemu => initialize_vm_connector(&inventory, &connector),
    }
}

fn initialize_pcileech(inventory: &Inventory, device: &str) -> Result<OsInstanceArcBox<'static>> {
    let args = Args::new().insert("device", device);

    let connector_args = ConnectorArgs::new(None, args, None);

    inventory
        .builder()
        .connector("pcileech")
        .args(connector_args)
        .os("win32")
        .build()
        .map_err(|e| anyhow::anyhow!(
            "PCILeech connector error: {e}\n\nCommon fixes:\n  1. Ensure FPGA device is properly connected\n  2. Check if PCILeech driver is installed\n  3. Run as Administrator"
        ))
}

fn find_target_process(
    os: OsInstanceArcBox<'static>,
) -> Result<IntoProcessInstanceArcBox<'static>> {
    let process = os.into_process_by_name(TARGET_PROCESS)?;
    Ok(process)
}

fn find_module_address(process: &mut IntoProcessInstanceArcBox<'_>) -> Result<Address> {
    let addr = process.module_by_name(TARGET_MODULE)?.base;
    Ok(addr)
}

// Not tested
fn initialize_vm_connector(
    inventory: &Inventory,
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
