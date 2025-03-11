mod initialization;
mod memory_layout;
mod stats;
mod test_runner;

use crate::connector::Connector;
use anyhow::Result;
use memflow::prelude::v1::*;
use std::{sync::Arc, time::Duration};

pub struct SpeedTest {
    process: Arc<parking_lot::RwLock<IntoProcessInstanceArcBox<'static>>>,
    test_addr: Address,
}

impl SpeedTest {
    pub fn new(connector: Connector, pcileech_device: String) -> Result<Self> {
        let (process, test_addr) =
            initialization::initialize_speedtest(connector, pcileech_device)?;
        let speedtest = Self {
            process: Arc::new(parking_lot::RwLock::new(process)),
            test_addr,
        };

        speedtest.print_memory_info()?;
        Ok(speedtest)
    }

    pub async fn run_speed_test(&self, duration: Duration) -> Result<()> {
        let mut runner = test_runner::TestRunner::new(self.process.clone(), self.test_addr);
        runner.run_tests(duration).await?;
        Ok(())
    }
}
