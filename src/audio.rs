// Copyright (C) 2025 Joshua Kesler
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{audio, messages::{AudioCommand, AudioDevice, AudioMessage}};
use crossbeam_channel::{Receiver, Sender};
use cpal::{StreamError, traits::{DeviceTrait, HostTrait, StreamTrait}};
use std::{thread, sync::{Arc, Mutex}};

pub fn start_thread(msg_tx: Sender<AudioMessage>, cmd_rx: Receiver<AudioCommand>) {
    thread::spawn(move || {
        let host: cpal::Host = cpal::default_host();
        let devices = match host.input_devices() {
            Ok(devs) => devs.collect::<Vec<_>>(),
            Err(e) => {
                let _ = msg_tx.send(AudioMessage::Error(format!("Audio host error: {}", e)));
                return;
            }
        };

        let device_list: Vec<AudioDevice> = devices.iter().enumerate().map(|(i, d)| {
            AudioDevice {
                name: d.name().unwrap_or_else(|_| format!("Unknown device: {}", i)),
                index: i
            }
        }).collect();

        if msg_tx.send(AudioMessage::DeviceList(device_list)).is_err() {
            return;
        }

        let mut active_stream: Option<cpal::Stream> = None;
        let mut selected_device_index = 0;
        let writer_handle = Arc::new(Mutex::new(None));
        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                AudioCommand::SelectDevice(index) => {
                    selected_device_index = index;
                    active_stream = None;
                },
                AudioCommand::StartRecording(filename) => {
                    let device = match devices.get(selected_device_index) {
                        Some(d) => d,
                        None => {
                            let _ = msg_tx.send(AudioMessage::Error(String::from("Invalid audio device index")));
                            continue;
                        }
                    };

                    let config = match device.default_input_config() {
                        Ok(c) => c,
                        Err(e) => {
                            let _ = msg_tx.send(AudioMessage::Error(format!("Failed to get microphone config: {}", e)));
                            continue;
                        }
                    };

                    let spec = hound::WavSpec {
                        channels: config.channels(),
                        sample_rate: config.sample_rate().0,
                        bits_per_sample: 32,
                        sample_format: hound::SampleFormat::Float
                    };

                    match hound::WavWriter::create(&filename, spec) {
                        Ok(writer) => {
                            if let Ok(mut guard) = writer_handle.lock() {
                                *guard = Some(writer);
                            } else {
                                let _ = msg_tx.send(AudioMessage::Error(String::from("Audio mutex poisoned")));
                                continue;
                            }

                            if active_stream.is_none() {
                                let writer_clone = writer_handle.clone();
                                let error_tx = msg_tx.clone();
                                let err_fn = move |err: StreamError| { let _ = error_tx.send(AudioMessage::Error(format!("Stream lost: {}", err))); };
                                let data_fn = move |data: &[f32], _: &_| {
                                    if let Ok(mut guard) = writer_clone.lock() {
                                        if let Some(writer) = guard.as_mut() {
                                            for &sample in data {
                                                let _ = writer.write_sample(sample);
                                            }
                                        }
                                    }
                                };

                                let stream_result = device.build_input_stream(&config.into(), data_fn, err_fn, None);
                                match stream_result {
                                    Ok(s) => {
                                        if let Err(e) = s.play() {
                                            let _ = msg_tx.send(AudioMessage::Error(format!("Failed to play stream: {}", e)));
                                        } else {
                                            active_stream = Some(s);
                                        }
                                    },
                                    Err(e) => {
                                        let _ = msg_tx.send(AudioMessage::Error(format!("Failed to build stream: {}", e)));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            let _ = msg_tx.send(AudioMessage::Error(format!("Could not create WAV file: {}", e)));
                        }
                    }
                },

                AudioCommand::StopRecording(ack_tx) => {
                    if let Ok(mut guard) = writer_handle.lock() {
                        if let Some(mut writer) = guard.take() {
                            if let Err(e) = writer.flush() {
                                let _ = msg_tx.send(AudioMessage::Error(format!("Failed to flush audio to disk: {}", e)));
                            }
                        }
                    }
                    
                    let _ = ack_tx.send(());
                }
            }
        }
    });
}