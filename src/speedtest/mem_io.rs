//! Retries for transient partial virtual DMA reads/writes.

use super::bench::BenchOp;
use memflow::error::PartialResult;
use memflow::prelude::v1::*;
use std::thread;
use std::time::Duration;

pub const MAX_IO_RETRIES: u32 = 5;

const INITIAL_BACKOFF: Duration = Duration::from_micros(250);
const MAX_BACKOFF: Duration = Duration::from_millis(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoAttempt {
    Ok,
    FailedAfterRetries,
}

pub fn read_raw_into_with_retry(
    process: &mut IntoProcessInstanceArcBox<'_>,
    addr: Address,
    buffer: &mut [u8],
) -> IoAttempt {
    retry_io(|| process.read_raw_into(addr, buffer))
}

pub fn write_raw_with_retry(
    process: &mut IntoProcessInstanceArcBox<'_>,
    addr: Address,
    data: &[u8],
) -> IoAttempt {
    retry_io(|| process.write_raw(addr, data))
}

fn retry_io(mut op: impl FnMut() -> PartialResult<()>) -> IoAttempt {
    let mut backoff = INITIAL_BACKOFF;

    for attempt in 0..MAX_IO_RETRIES {
        match op() {
            Ok(()) => return IoAttempt::Ok,
            Err(_) if attempt + 1 < MAX_IO_RETRIES => {
                thread::sleep(backoff);
                backoff = (backoff * 2).min(MAX_BACKOFF);
            }
            Err(_) => {}
        }
    }

    IoAttempt::FailedAfterRetries
}

pub fn retry_exhausted_message(op: BenchOp, retries: u32) -> String {
    let kind = match op {
        BenchOp::Read => "read",
        BenchOp::Write => "write",
    };
    format!("DMA {kind} failed after {retries} retries ({kind} may be transient; op skipped)")
}
