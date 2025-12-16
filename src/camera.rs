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

use crate::messages::{camera::{CameraCommand, CameraMessage}, recorder::RecorderCommand, video::VideoConfig};
use crossbeam_channel::{Sender, Receiver};
use image::imageops::FilterType;
use nokhwa::{Camera, pixel_format::RgbFormat, utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType}};
use std::{sync::{Arc, Mutex}, thread, time::{Duration, Instant}};

const MJPEG: &str = "MJPEG";
const YUYV: &str = "YUYV";
const NV12: &str = "NV12";
const GRAY: &str = "GRAY";
const W480p: u32 = 854;
const H480p: u32 = 480;

pub fn start_thread(tx: Sender<CameraMessage>, rec_tx: Sender<RecorderCommand>, cmd_rx: Receiver<CameraCommand>) {
    thread::spawn(move || {
        loop {
            let index: CameraIndex = CameraIndex::Index(0);
            let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
            let query_camera_result = Camera::new(index.clone(), requested);
            match query_camera_result {
                Ok(mut camera) => {
                    match camera.compatible_camera_formats() {
                        Ok(formats) => {
                            let mut configs = Vec::new();
                            for fmt in formats {
                                let c = VideoConfig {
                                    width: fmt.resolution().width(),
                                    height: fmt.resolution().height(),
                                    fps: fmt.frame_rate(),
                                    fmt: fmt.format().to_string()
                                };
                                if !configs.contains(&c) { configs.push(c); }
                            }
                            configs.sort_by(|a, b| b.width.cmp(&a.width).then(b.fps.cmp(&a.fps)));
                            let _ = tx.send(CameraMessage::Capabilities(configs));
                        },
                        Err(e) => {
                            let _ = tx.send(CameraMessage::Error(format!("Query failed: {}", e)));
                            if wait_for_retry(&cmd_rx) { continue; } else { break; }
                        }
                    }
                    
                    drop(camera);
                },
                Err(e) => {
                    let _= tx.send(CameraMessage::Error(format!("Camera initialization failed: {}", e)));
                    if wait_for_retry(&cmd_rx) { continue; } else { break; }
                }
            };

            let cfg = match cmd_rx.recv() {
                Ok(CameraCommand::StartStream(c)) => c,
                Ok(CameraCommand::Retry) => continue,
                Err(_) => break
            };

            let frame_format = match cfg.fmt.as_str() {
                MJPEG => FrameFormat::MJPEG,
                YUYV => FrameFormat::YUYV,
                _ => FrameFormat::MJPEG
            };
            let exact = CameraFormat::new_from(cfg.width, cfg.height, frame_format, cfg.fps);
            println!("camera line 76) cfg.fps: {}", cfg.fps.to_string());
            let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(exact));
            let mut camera = match Camera::new(index, req) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(CameraMessage::Error(format!("Re-init failed: {}", e)));
                    if wait_for_retry(&cmd_rx) { continue; } else { break; }
                }
            };

            if let Err(e) = camera.open_stream() {
                let _ = tx.send(CameraMessage::Error(format!("Open stream failed: {}", e)));
                if wait_for_retry(&cmd_rx) { continue; } else { break; }
            }

            let _ = tx.send(CameraMessage::StreamStarted(cfg.width, cfg.height, cfg.fps));
            let latest_frame: Arc<Mutex<Option<Arc<Vec<u8>>>>> = Arc::new(Mutex::new(None));
            let cap_frame_storage = latest_frame.clone();
            let ui_tx = tx.clone();

            thread::spawn(move || {
                loop {
                    match camera.frame() {
                        Ok(frame) => {
                            let raw_data = frame.buffer().to_vec();
                            let raw_arc = Arc::new(raw_data);
                            if let Ok(mut guard) = cap_frame_storage.lock() {
                                *guard = Some(raw_arc.clone());
                            }

                            if let Ok(decoded) = frame.decode_image::<RgbFormat>() {
                                let preview = image::imageops::resize(&decoded, W480p, H480p, FilterType::Nearest);
                                let p_width = preview.width();
                                let p_height = preview.height();
                                let preview = preview.into_raw();
                                let raw: Arc<Vec<u8>> = Arc::new(vec![]);
                                let _ = ui_tx.send(CameraMessage::Frame {
                                    raw,
                                    preview,
                                    p_width,
                                    p_height
                                });
                            }
                        },
                        Err(_) => {
                            thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
            });

            let target_interval = Duration::from_secs_f64(1.0/cfg.fps as f64);
            let mut next_tick = Instant::now();

            loop {
                let frame_to_send = {
                    let guard = latest_frame.lock().unwrap();
                    guard.clone()
                };

                if let Some(data) = frame_to_send {
                    let capture_time = Instant::now();
                    let _ = rec_tx.send(RecorderCommand::WriteFrame(data, capture_time));
                }

                next_tick += target_interval;
                let now = Instant::now();
                if next_tick > now {
                    thread::sleep(next_tick - now);
                } else {
                    next_tick = now;
                }
            }
        }
    });
}

fn wait_for_retry(rx: &Receiver<CameraCommand>) -> bool {
    loop {
        match rx.recv() {
            Ok(CameraCommand::Retry) => return true,
            Ok(_) => {},
            Err(_) => return false
        }
    }
}