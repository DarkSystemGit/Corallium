use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use midly::{Smf, TrackEventKind, MidiMessage, MetaMessage, Timing};

pub const CHANNELS_WAVEFORMS: [&str; 8] = [
    "square", "square", "square", "square",
    "saw", "saw",
    "triangle", "triangle"
];

const DEFAULT_PANS: [f32; 8] = [-0.5, 0.5, -0.2, 0.2, -0.7, 0.7, -0.4, 0.4];

#[derive(Debug, Clone)]
pub struct Event {
    pub timestamp_ms: i32,
    pub freq: f32,
    pub vol: i16,
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub pan: f32,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub struct Song {
    pub channels: [Channel; 8],
}

struct FlattenedEvent<'a> {
    absolute_tick: u32,
    kind: TrackEventKind<'a>,
}

pub fn process_midi(midi_bytes: &[u8], wav_export_path: Option<&str>) -> Result<Vec<u8>, String> {
    let smf = Smf::parse(midi_bytes).map_err(|e| format!("MIDI parse error: {:?}", e))?;

    let tpq = match smf.header.timing {
        Timing::Metrical(ticks_per_beat) => ticks_per_beat.as_int() as f64,
        _ => 480.0,
    };

    let mut flat_events = Vec::new();
    for track in smf.tracks.iter() {
        let mut current_tick = 0u32;
        for event in track.iter() {
            let delta = u32::from(event.delta);
            current_tick += delta;
            flat_events.push(FlattenedEvent {
                absolute_tick: current_tick,
                kind: event.kind.clone(),
            });
        }
    }

    flat_events.sort_by(|a, b| {
        if a.absolute_tick == b.absolute_tick {
            let a_is_meta = matches!(a.kind, TrackEventKind::Meta(_));
            let b_is_meta = matches!(b.kind, TrackEventKind::Meta(_));
            b_is_meta.cmp(&a_is_meta)
        } else {
            a.absolute_tick.cmp(&b.absolute_tick)
        }
    });

    let mut temp_channels_data: Vec<Vec<(i32, f32, i16)>> = vec![Vec::new(); 8];
    let mut channel_states: Vec<Option<((u8, u8), i32, f32)>> = vec![None; 8];

    let mut current_tick = 0u32;
    let mut current_ms = 0.0f64;
    let mut tempo_us = 500_000.0f64;
    let mut ms_per_tick = tempo_us / (1000.0 * tpq);

    for flat_event in &flat_events {
        let delta_ticks = flat_event.absolute_tick - current_tick;
        if delta_ticks > 0 {
            current_ms += delta_ticks as f64 * ms_per_tick;
            current_tick = flat_event.absolute_tick;
        }

        let timestamp_ms = current_ms.round() as i32;

        match &flat_event.kind {
            TrackEventKind::Meta(MetaMessage::Tempo(t)) => {
                tempo_us = t.as_int() as f64;
                ms_per_tick = tempo_us / (1000.0 * tpq);
            }
            TrackEventKind::Midi { channel, message } => {
                let ch_val = channel.as_int();
                if ch_val == 9 {
                    continue;
                }

                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        let key_val = key.as_int();
                        let vel_val = vel.as_int();
                        let note_key = (ch_val, key_val);

                        if vel_val > 0 {
                            // Inlined Octave Transpose
                            let mut corallium_note = key_val;
                            while corallium_note < 36 { corallium_note += 12; }
                            while corallium_note > 99 { corallium_note -= 12; }

                            // Inlined Frequency Calculation
                            let freq = 440.0 * 2.0f32.powf((corallium_note as f32 - 69.0) / 12.0);
                            let vol = ((vel_val as f32 / 127.0) * 32767.0) as i16;

                            for ch in 0..8 {
                                if let Some((active_key, _, active_freq)) = channel_states[ch] {
                                    if active_key == note_key {
                                        temp_channels_data[ch].push((timestamp_ms, active_freq, 0));
                                        channel_states[ch] = None;
                                    }
                                }
                            }

                            // Inlined Channel Priorities
                            let priority = if corallium_note >= 55 {
                                [0, 1, 2, 3, 6, 7, 4, 5]
                            } else {
                                [6, 7, 4, 5, 0, 1, 2, 3]
                            };

                            let mut allocated_channel = None;
                            for &ch in &priority {
                                if channel_states[ch].is_none() {
                                    allocated_channel = Some(ch);
                                    break;
                                }
                            }

                            if allocated_channel.is_none() {
                                let mut oldest_time = i32::MAX;
                                let mut steal_ch = None;
                                for &ch in &priority {
                                    if let Some((_, start_time, _)) = channel_states[ch] {
                                        if start_time < oldest_time {
                                            oldest_time = start_time;
                                            steal_ch = Some(ch);
                                        }
                                    }
                                }
                                if let Some(ch) = steal_ch {
                                    if let Some((_, _, active_freq)) = channel_states[ch] {
                                        temp_channels_data[ch].push((timestamp_ms, active_freq, 0));
                                    }
                                    channel_states[ch] = None;
                                    allocated_channel = Some(ch);
                                }
                            }

                            if let Some(ch) = allocated_channel {
                                channel_states[ch] = Some((note_key, timestamp_ms, freq));
                                temp_channels_data[ch].push((timestamp_ms, freq, vol));
                            }
                        } else {
                            for ch in 0..8 {
                                if let Some((active_key, _, active_freq)) = channel_states[ch] {
                                    if active_key == note_key {
                                        temp_channels_data[ch].push((timestamp_ms, active_freq, 0));
                                        channel_states[ch] = None;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        let key_val = key.as_int();
                        let note_key = (ch_val, key_val);
                        for ch in 0..8 {
                            if let Some((active_key, _, active_freq)) = channel_states[ch] {
                                if active_key == note_key {
                                    temp_channels_data[ch].push((timestamp_ms, active_freq, 0));
                                    channel_states[ch] = None;
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    let last_time_ms = current_ms.round() as i32;
    for ch in 0..8 {
        if let Some((_, _, active_freq)) = channel_states[ch] {
            temp_channels_data[ch].push((last_time_ms + 50, active_freq, 0));
        }
    }

    let mut compiled_channels = [
        Channel { pan: DEFAULT_PANS[0], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[1], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[2], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[3], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[4], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[5], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[6], events: Vec::new() },
        Channel { pan: DEFAULT_PANS[7], events: Vec::new() },
    ];

    // Inlined Event Deduplication and Filtering
    for ch in 0..8 {
        let events = &temp_channels_data[ch];
        if !events.is_empty() {
            let mut time_groups: BTreeMap<i32, (f32, i16)> = BTreeMap::new();
            for &(t, freq, vol) in events {
                if let Some(existing) = time_groups.get_mut(&t) {
                    if vol > existing.1 {
                        *existing = (freq, vol);
                    }
                } else {
                    time_groups.insert(t, (freq, vol));
                }
            }

            let mut filtered = Vec::new();
            let mut last_vol = -1i16;
            let mut last_freq = -1.0f32;

            for (t, (freq, vol)) in time_groups {
                if vol != last_vol || (vol > 0 && (freq - last_freq).abs() > 1e-4) {
                    filtered.push(Event { timestamp_ms: t, freq, vol });
                    last_vol = vol;
                    last_freq = freq;
                }
            }
            compiled_channels[ch].events = filtered;
        }
    }

    let song = Song { channels: compiled_channels };

    if let Some(wav_path) = wav_export_path {
        song.render_wav(wav_path, 44100).map_err(|e| format!("WAV render error: {:?}", e))?;
    }

    Ok(song.to_bytes())
}

pub fn convert_music(input_midi_file: &str, wav_preview: bool) -> Result<(), String> {
    let midi_bytes = std::fs::read(input_midi_file)
        .map_err(|e| format!("Failed to read MIDI file '{}': {}", input_midi_file, e))?;

    let output_path = {
        let mut path = PathBuf::from(input_midi_file);
        path.set_extension("csfx");
        path
    };

    let wav_export_path = if wav_preview {
        let mut wav_path = output_path.clone();
        wav_path.set_extension("wav");
        Some(wav_path)
    } else {
        None
    };

    let wav_export_path = wav_export_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());

    let converted = process_midi(&midi_bytes, wav_export_path.as_deref())?;
    std::fs::write(&output_path, converted).map_err(|e| {
        format!(
            "Failed to write converted output '{}': {}",
            output_path.display(),
            e
        )
    })?;
    Ok(())
}

impl Song {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write 48-byte Header
        for channel in &self.channels {
            bytes.extend_from_slice(&channel.pan.to_le_bytes());
            let event_count = channel.events.len() as i16;
            bytes.extend_from_slice(&event_count.to_le_bytes());
        }

        // Write Event Body
        for channel in &self.channels {
            for event in &channel.events {
                bytes.extend_from_slice(&event.timestamp_ms.to_le_bytes());
                bytes.extend_from_slice(&event.freq.to_le_bytes());
                bytes.extend_from_slice(&event.vol.to_le_bytes());
            }
        }

        bytes
    }

    pub fn render_wav(&self, path: &str, sample_rate: u32) -> Result<(), std::io::Error> {
        // Inlined Duration Scan
        let total_duration_ms = self.channels
            .iter()
            .flat_map(|ch| ch.events.iter())
            .map(|ev| ev.timestamp_ms)
            .max()
            .unwrap_or(0);

        let total_samples = ((total_duration_ms as f64 / 1000.0) * sample_rate as f64).round() as usize;
        let mut master_buffer = vec![0.0f32; total_samples * 2];

        for (ch_idx, channel) in self.channels.iter().enumerate() {
            if channel.events.is_empty() {
                continue;
            }

            let wave_type = CHANNELS_WAVEFORMS[ch_idx];

            let mut spans = Vec::new();
            for i in 0..channel.events.len() {
                let start_ms = channel.events[i].timestamp_ms;
                let end_ms = if i + 1 < channel.events.len() {
                    channel.events[i + 1].timestamp_ms
                } else {
                    total_duration_ms
                };

                let start_sample = ((start_ms as f64 / 1000.0) * sample_rate as f64).round() as usize;
                let end_sample = ((end_ms as f64 / 1000.0) * sample_rate as f64).round() as usize;

                if end_sample > start_sample {
                    spans.push((
                        start_sample,
                        end_sample,
                        channel.events[i].freq,
                        channel.events[i].vol as f32 / 32767.0,
                    ));
                }
            }

            let mut phase = 0.0f32;
            for (start, end, freq, vol) in spans {
                if vol <= 0.0 || freq <= 0.0 {
                    phase = 0.0;
                    continue;
                }

                let phase_increment = freq / sample_rate as f32;
                let left_gain = (1.0 - channel.pan) / 2.0;
                let right_gain = (1.0 + channel.pan) / 2.0;

                for sample_idx in start..end {
                    let buffer_idx = sample_idx * 2;
                    if buffer_idx + 1 >= master_buffer.len() {
                        break;
                    }

                    let y = match wave_type {
                        "square" => {
                            if phase < 0.5 { 1.0 } else { -1.0 }
                        }
                        "saw" => {
                            2.0 * phase - 1.0
                        }
                        "triangle" => {
                            if phase < 0.5 {
                                4.0 * phase - 1.0
                            } else {
                                3.0 - 4.0 * phase
                            }
                        }
                        _ => 0.0,
                    };

                    master_buffer[buffer_idx] += y * vol * left_gain * 0.1;
                    master_buffer[buffer_idx + 1] += y * vol * right_gain * 0.1;

                    phase = (phase + phase_increment) % 1.0;
                }
            }
        }

        // ==========================================
        // 12dB/Octave Biquad Low-Pass Filter (RBJ)
        // Cuts harsh digital frequencies above 2200Hz
        // ==========================================
        if !master_buffer.is_empty() {
            let w0 = 2.0 * std::f32::consts::PI * 2200.0 / sample_rate as f32;
            let cos_w0 = w0.cos();
            let sin_w0 = w0.sin();
            let q = 0.7071f32; // Butterworth response
            let alpha = sin_w0 / (2.0 * q);

            let b0 = (1.0 - cos_w0) / 2.0;
            let b1 = 1.0 - cos_w0;
            let b2 = (1.0 - cos_w0) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_w0;
            let a2 = 1.0 - alpha;

            let nb0 = b0 / a0;
            let nb1 = b1 / a0;
            let nb2 = b2 / a0;
            let na1 = a1 / a0;
            let na2 = a2 / a0;

            // Stereo filter state registers
            let mut x1_l = 0.0f32;
            let mut x2_l = 0.0f32;
            let mut y1_l = 0.0f32;
            let mut y2_l = 0.0f32;

            let mut x1_r = 0.0f32;
            let mut x2_r = 0.0f32;
            let mut y1_r = 0.0f32;
            let mut y2_r = 0.0f32;

            for i in (0..master_buffer.len()).step_by(2) {
                // Filter Left Channel
                let x_l = master_buffer[i];
                let y_l = nb0 * x_l + nb1 * x1_l + nb2 * x2_l - na1 * y1_l - na2 * y2_l;
                x2_l = x1_l;
                x1_l = x_l;
                y2_l = y1_l;
                y1_l = y_l;
                master_buffer[i] = y_l;

                // Filter Right Channel
                let x_r = master_buffer[i + 1];
                let y_r = nb0 * x_r + nb1 * x1_r + nb2 * x2_r - na1 * y1_r - na2 * y2_r;
                x2_r = x1_r;
                x1_r = x_r;
                y2_r = y1_r;
                y1_r = y_r;
                master_buffer[i + 1] = y_r;
            }
        }

        // Convert the filtered floats to 16-bit PCM bytes
        let mut pcm_data = Vec::with_capacity(master_buffer.len() * 2);
        for &sample in &master_buffer {
            let clipped = sample.clamp(-1.0, 1.0);
            let pcm_val = (clipped * 32767.0) as i16;
            pcm_data.extend_from_slice(&pcm_val.to_le_bytes());
        }

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        let num_channels = 2u16;
        let bits_per_sample = 16u16;
        let byte_rate = sample_rate * num_channels as u32 * bits_per_sample as u32 / 8;
        let block_align = num_channels * bits_per_sample / 8;
        let subchunk2_size = pcm_data.len() as u32;
        let chunk_size = 36 + subchunk2_size;

        writer.write_all(b"RIFF")?;
        writer.write_all(&chunk_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?;
        writer.write_all(&1u16.to_le_bytes())?;
        writer.write_all(&num_channels.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&bits_per_sample.to_le_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&subchunk2_size.to_le_bytes())?;
        writer.write_all(&pcm_data)?;

        writer.flush()?;
        Ok(())
    }
}
