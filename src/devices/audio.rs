use crate::util::flatten_vec;
use crate::util::pop_stack;
use crate::util::unpack_float;
use crate::vm::Machine;
use arc_swap::{ArcSwap, ArcSwapAny};
use hound;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::{
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering::Relaxed},
    },
    vec,
};
use tinyaudio::prelude::*;

use super::RawDevice;
//4 square, 2 triangle, 2 sawtooth, 2 sample
pub fn driver(machine: &mut Machine, command: i16, device_id: usize) {
    match command {
        0 => {
            //pause()
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                audio.pause();
            }
            if machine.debug {
                println!("IO.audio.pause");
            }
        }
        1 => {
            //unpause()
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                audio.unpause();
            }
            if machine.debug {
                println!("IO.audio.unpause");
            }
        }
        2 => {
            //changeVolume(channel,newVolume)
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                let args = pop_stack(&mut machine.core, 2);
                let channel = args[0] as usize;
                let new_volume = args[1] as f32;
                audio.update_channel(channel, ChannelUpdate::Volume(new_volume));
                if machine.debug {
                    println!("IO.audio.changeVolume {} {}", channel, new_volume);
                }
            }
        }
        3 => {
            //changePan(channel,newPan)
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                let args = pop_stack(&mut machine.core, 3);
                let channel = args[0] as usize;
                let new_pan = [args[1] as f32, args[2] as f32];
                audio.update_channel(channel, ChannelUpdate::Pan(new_pan));
                if machine.debug {
                    println!(
                        "IO.audio.changePan {} [L: {}, R: {}]",
                        channel, args[1], args[2]
                    );
                }
            }
        }
        4 => {
            //changeFrequency(channel,newFrequency)
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                let args = pop_stack(&mut machine.core, 2);
                let channel = args[0] as usize;
                let new_frequency = args[1] as f32;
                audio.update_channel(channel, ChannelUpdate::Frequency(new_frequency));
                if machine.debug {
                    println!("IO.audio.changeFrequency {} {}", channel, new_frequency);
                }
            }
        }
        5 => {
            //changeMasterVolume(newVolume)
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                let args = pop_stack(&mut machine.core, 1);
                let new_volume = args[0] as i32;
                audio.set_master_volume(new_volume);
                if machine.debug {
                    println!("IO.audio.changeMasterVolume {}", new_volume);
                }
            }
        }
        6 => {
            //loadSound(channel, ptr, len)
            let args = pop_stack(&mut machine.core, 3);
            let channel = args[0] as usize;
            let ptr = args[1] as usize;
            let len = args[2] as usize;
            //dbg!(ptr, len, channel);
            let rdata = machine.memory.read_range(ptr..ptr + len, machine).to_vec();
            let data: Vec<f32> = rdata
                .iter()
                .enumerate()
                .step_by(2)
                .map(|(i, x)| {
                    unpack_float(&[*x, rdata[i + 1]]).expect("Couldn't parse sample float")
                })
                .collect();
            //println!("{:?}", rdata);
            if let RawDevice::Audio(audio) = &mut machine.devices[device_id].contents {
                audio.update_channel(channel, ChannelUpdate::WaveSample(data));
                if machine.debug {
                    println!("IO.audio.loadSound {} %[{}..{}]", channel, ptr, ptr + len);
                }
            }
        }
        _ => {}
    }
}
pub struct AudioDevice {
    channels: ChannelCollection,
    pub sample_rate: u32,
    old_vol: f32,
    master_volume: Arc<AtomicI32>,
    device: Option<OutputDevice>,
    updates: Sender<(usize, ChannelUpdate)>,
}
impl std::fmt::Debug for AudioDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioDevice")
            .field("channels", &self.channels)
            .field("sample_rate", &self.sample_rate)
            .field("old_vol", &self.old_vol)
            .field("master_volume", &self.master_volume)
            .finish()
    }
}
impl AudioDevice {
    pub fn new() -> AudioDevice {
        let (tx, rx) = channel::<(usize, ChannelUpdate)>();
        let mut a = AudioDevice {
            channels: mutex_channels(flatten_vec(vec![
                gen_uninitialized_channels(gen_square_wave as WaveGenerator, 4, 10.0),
                gen_uninitialized_channels(gen_triangle_wave as WaveGenerator, 2, 10.0),
                gen_uninitialized_channels(gen_sawtooth_wave as WaveGenerator, 2, 10.0),
                vec![
                    Channel::new_with_sample(vec![0.0], 0.1, [1.0, 1.0]),
                    Channel::new_with_sample(vec![0.0], 0.1, [1.0, 1.0]),
                ],
            ])),
            sample_rate: 32000,
            old_vol: 1.0,
            master_volume: Arc::new(AtomicI32::new(100)),
            device: None,
            updates: tx,
        };
        a.run(rx);
        a
    }
    pub fn update_channel(&self, id: usize, update: ChannelUpdate) {
        self.updates.send((id, update)).err();
        //modify_channel_collection_item(id, &(self.channels), update);
    }

    pub fn set_master_volume(&self, volume: i32) {
        self.master_volume.store(volume, Relaxed);
    }
    pub fn run(&mut self, rx: Receiver<(usize, ChannelUpdate)>) {
        let params = OutputDeviceParameters {
            channels_count: 2,
            sample_rate: self.sample_rate as usize,
            channel_sample_count: (self.sample_rate / 10) as usize,
        };
        self.device = Some(
            run_output_device(
                params,
                gen_wave(
                    params.sample_rate as u32,
                    Arc::clone(&(self.channels)),
                    Arc::clone(&self.master_volume),
                    rx,
                ),
            )
            .expect("Could not initialize audio device"),
        );
    }
    pub fn pause(&mut self) {
        self.old_vol = self.master_volume.load(Relaxed) as f32 / 100.0;
        self.master_volume.store(0, Relaxed);
    }
    pub fn unpause(&mut self) {
        self.master_volume
            .store((self.old_vol * 100.0) as i32, Relaxed);
    }
}
fn gen_uninitialized_channels(wave: WaveGenerator, count: i32, ttl: f32) -> Vec<Channel> {
    let mut r = vec![];
    for _i in 0..count {
        r.push(Channel::new(0.0, 1.0 / ttl, [1.0, 1.0], wave));
    }
    r
}
type ChannelCollection = Arc<Vec<ArcSwapAny<Arc<Channel>>>>;
fn mutex_channels(channels: Vec<Channel>) -> ChannelCollection {
    Arc::new(
        channels
            .iter()
            .map(|c| ArcSwap::from(Arc::new(c.clone())))
            .collect(),
    )
}
pub fn load_wav(bytes: &[u8]) -> Vec<f32> {
    let mut reader = hound::WavReader::new(Cursor::new(bytes)).expect("Couldn't parse WAV sample");
    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader.samples::<f32>().map(|s| s.unwrap()).collect(),
    };
    if spec.channels == 2 {
        // Average every two samples (L and R) into one
        samples
            .chunks_exact(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        samples
    }
}

fn modify_channel_collection_item(id: usize, channels: &ChannelCollection, update: ChannelUpdate) {
    if let Some(channel) = channels.get(id) {
        let uc = channel.load();
        if uc.freq != None {
            match update {
                ChannelUpdate::Volume(volume) => {
                    channel.store(Arc::new(Channel::new(
                        uc.freq.unwrap(),
                        volume / 10.0,
                        uc.pan,
                        uc.wave.unwrap(),
                    )));
                }
                ChannelUpdate::Pan(pan) => {
                    channel.store(Arc::new(Channel::new(
                        uc.freq.unwrap(),
                        uc.volume,
                        pan,
                        uc.wave.unwrap(),
                    )));
                }
                ChannelUpdate::Frequency(freq) => {
                    channel.store(Arc::new(Channel::new(
                        freq,
                        uc.volume,
                        uc.pan,
                        uc.wave.unwrap(),
                    )));
                }
                _ => {}
            }
        } else {
            match update {
                ChannelUpdate::Volume(volume) => {
                    channel.store(Arc::new(Channel::new_with_sample(
                        uc.wave_sample.clone().unwrap(),
                        volume / 10.0,
                        uc.pan,
                    )));
                }
                ChannelUpdate::Pan(pan) => {
                    channel.store(Arc::new(Channel::new_with_sample(
                        uc.wave_sample.clone().unwrap(),
                        uc.volume,
                        pan,
                    )));
                }
                ChannelUpdate::WaveSample(sample) => {
                    channel.store(Arc::new(Channel::new_with_sample(
                        sample, uc.volume, uc.pan,
                    )));
                }
                _ => {}
            }
        }
    }
}
pub enum ChannelUpdate {
    Volume(f32),
    Pan([f32; 2]),
    Frequency(f32),
    WaveSample(Vec<f32>),
}
type WaveGenerator = fn(f32, f32) -> f32;
fn gen_wave(
    sample_rate: u32,
    channels: ChannelCollection,
    master_volume: Arc<AtomicI32>,
    rx: Receiver<(usize, ChannelUpdate)>,
) -> impl FnMut(&mut [f32]) {
    let mut channel_clocks: Vec<f32> = vec![0.0; channels.len()];
    let mut loaded_channels = Vec::with_capacity(10);
    move |data| {
        for (id, update) in rx.try_iter().take(50) {
            let reset_clock = matches!(&update, ChannelUpdate::WaveSample(_));
            modify_channel_collection_item(id, &channels, update);
            if reset_clock && id < channel_clocks.len() {
                channel_clocks[id] = 0.0;
            }
        }
        loaded_channels.clear();
        for channel in channels.iter() {
            loaded_channels.push(channel.load());
        }
        let volume = (master_volume.load(Relaxed) as f32) / 100.0;
        for samples in data.chunks_exact_mut(2) {
            let mut value_l = 0.0;
            let mut value_r = 0.0;
            for (i, channel) in loaded_channels.iter().enumerate() {
                let raw_val = if let Some(freq) = channel.freq {
                    if freq > 0.0 {
                        // Increment phase: (frequency / sample_rate) gives the % of the cycle per sample
                        channel_clocks[i] = (channel_clocks[i] + freq / sample_rate as f32) % 1.0;
                    }
                    channel.play(channel_clocks[i])
                } else if let Some(sample) = &channel.wave_sample {
                    let val = channel.play(channel_clocks[i]);
                    if !sample.is_empty() && channel_clocks[i] < sample.len() as f32 {
                        channel_clocks[i] += 1.0;
                    }
                    val
                } else {
                    channel.play(channel_clocks[i])
                };
                value_l += raw_val * channel.pan[0];
                value_r += raw_val * channel.pan[1];
            }
            samples[0] = clamp(value_l * volume);
            samples[1] = clamp(value_r * volume);
        }
    }
}
fn clamp(val: f32) -> f32 {
    if val > 1.0 {
        1.0
    } else if val < -1.0 {
        -1.0
    } else {
        val
    }
}
fn gen_square_wave(phase: f32, volume: f32) -> f32 {
    (if phase < 0.5 { 1.0 } else { -1.0 }) * volume
}

fn gen_triangle_wave(phase: f32, volume: f32) -> f32 {
    (2.0 * (2.0 * phase - 1.0).abs() - 1.0) * volume
}

fn gen_sawtooth_wave(phase: f32, volume: f32) -> f32 {
    (2.0 * phase - 1.0) * volume
}

#[derive(Debug, Clone)]
struct Channel {
    freq: Option<f32>,
    volume: f32,
    wave: Option<WaveGenerator>,
    pan: [f32; 2],
    wave_sample: Option<Vec<f32>>,
}

impl Channel {
    fn new(freq: f32, volume: f32, pan: [f32; 2], wave: WaveGenerator) -> Self {
        Channel {
            freq: Some(freq),
            volume,
            wave: Some(wave),
            pan,
            wave_sample: None,
        }
    }
    fn new_with_sample(sample: Vec<f32>, volume: f32, pan: [f32; 2]) -> Self {
        Channel {
            freq: None,
            volume,
            wave: None,
            pan,
            wave_sample: Some(sample),
        }
    }
    fn play(&self, phase: f32) -> f32 {
        if let Some(wave_func) = self.wave {
            wave_func(phase, self.volume)
        } else if let Some(sample) = &self.wave_sample {
            // phase is the index for raw samples
            if phase < (sample.len() as f32) {
                sample[phase as usize] * self.volume
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}
