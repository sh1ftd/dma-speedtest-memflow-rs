use super::*;

const CORE_MODULES: [&str; 4] = ["ntdll.dll", "kernel32.dll", "kernelbase.dll", "win32u.dll"];

const SEPARATOR_LINE: &str = "--------------------------------------------------";

impl SpeedTest {
    pub(super) fn print_memory_info(&self) -> Result<()> {
        self.enumerate_modules()?;
        self.get_physical_map()?;
        Ok(())
    }

    fn enumerate_modules(&self) -> Result<()> {
        println!("\nKey System Modules:");
        println!("{}", SEPARATOR_LINE);
        println!("Base Address       Size      Name");
        println!("{}", SEPARATOR_LINE);

        let mut process = self.process.write();
        let module_list = process.module_list()?;

        module_list
            .into_iter()
            .filter(|module| CORE_MODULES.contains(&module.name.to_lowercase().as_str()))
            .for_each(|module| {
                println!(
                    "{:#016x}   {:#8x}   {}",
                    module.base, module.size, module.name
                );
            });

        println!("{}", SEPARATOR_LINE);
        Ok(())
    }

    fn get_physical_map(&self) -> Result<()> {
        println!("\nPhysical Memory Information:");
        println!("{}", SEPARATOR_LINE);

        let mut process = self.process.write();
        let phys_mem = process.phys_mem();
        let metadata = phys_mem.metadata();

        println!("Physical Size: {:#010x} bytes", metadata.real_size);
        println!("Max Address:   {:#016x}", metadata.max_address);

        println!("{}", SEPARATOR_LINE);
        Ok(())
    }
}
