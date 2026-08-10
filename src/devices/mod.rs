use crate::devices::audio::AudioDevice;
use crate::devices::clock::Clock;
use crate::devices::disk::Disk;
use crate::devices::gfx::GraphicsSystem;
use crate::vm::Machine;
pub mod audio;
pub mod clock;
pub mod disk;
pub mod gfx;
pub mod serial;
#[derive(Debug)]
pub struct Device {
    pub driver: fn(machine: &mut Machine, command: i16, device_id: usize),
    pub contents: RawDevice,
}
#[derive(Debug)]
pub enum RawDevice {
    Disk(Disk),
    Audio(AudioDevice),
    Clock(Clock),
    Graphics(GraphicsSystem),
    Serial,
}
pub fn get_device_list() -> Vec<Device> {
    vec![
        Device {
            driver: disk::driver,
            contents: RawDevice::Disk(Disk::new()),
        },
        Device {
            driver: audio::driver,
            contents: RawDevice::Audio(AudioDevice::new()),
        },
        Device {
            driver: clock::driver,
            contents: RawDevice::Clock(Clock::new()),
        },
        Device {
            driver: gfx::driver,
            contents: RawDevice::Graphics(GraphicsSystem::new([320, 240])),
        },
        Device {
            driver: serial::driver,
            contents: RawDevice::Serial,
        },
    ]
}
pub fn get_reset_device_list(gfx: RawDevice, disk: RawDevice) -> Vec<Device> {
    // Reuse the provided display and disk contents by taking ownership of
    // the supplied RawDevice values.
    let display = match gfx {
        RawDevice::Graphics(gs) => gs.display,
        _ => panic!("internal error fetching display"),
    };
    let disk_contents = match disk {
        RawDevice::Disk(d) => d,
        _ => panic!("internal error fetching disk"),
    };
    vec![
        Device {
            driver: disk::driver,
            contents: RawDevice::Disk(disk_contents),
        },
        Device {
            driver: audio::driver,
            contents: RawDevice::Audio(AudioDevice::new()),
        },
        Device {
            driver: clock::driver,
            contents: RawDevice::Clock(Clock::new()),
        },
        Device {
            driver: gfx::driver,
            contents: RawDevice::Graphics(GraphicsSystem::new_with_display(
                display,
                [320, 240],
            )),
        },
        Device {
            driver: serial::driver,
            contents: RawDevice::Serial,
        },
    ]
}
