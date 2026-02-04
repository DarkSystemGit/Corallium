use crate::devices::audio::AudioDevice;
use crate::devices::clock::Clock;
use crate::devices::disk::Disk;
use crate::devices::gfx::GraphicsSystem;
use crate::vm::Machine;
pub mod audio;
pub mod clock;
pub mod disk;
pub mod gfx;
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
    ]
}
