use crate::devices::RawDevice;
use crate::vm::{DataType, Machine};
use std::time::{SystemTime, UNIX_EPOCH};
#[derive(Debug)]
pub struct Clock {
    start: u64,
}

impl Clock {
    pub fn new() -> Self {
        Clock {
            start: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Couldn't get time")
                .as_secs(),
        }
    }
    fn read(&self) -> i32 {
        (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Couldn't get time")
            .as_secs()
            - self.start) as i32
    }
}
pub fn driver(machine: &mut Machine, command: i16, device_id: usize) {
    match command {
        0 => {
            if let RawDevice::Clock(clock) = &machine.devices[device_id].contents {
                machine
                    .core
                    .stack
                    .push(DataType::Int32(clock.read()), &mut machine.core.srp);
                if machine.debug {
                    println!("IO.clock.read");
                }
            } else {
                machine
                    .core
                    .stack
                    .push(DataType::Int32(0), &mut machine.core.srp);
            }
        }
        _ => {}
    }
}
