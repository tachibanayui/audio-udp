pub mod sigoid;
pub mod throughput_meter;
pub mod console_status;

pub use sigoid::{*};
pub use throughput_meter::{*};
pub use console_status::{*};

use std::time::Duration;
pub fn to_reference_time(d: Duration) -> i64 {
    (d.as_nanos() / 100) as i64
}
