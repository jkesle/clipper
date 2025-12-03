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

use crate::messages::{camera::{CameraCommand, CameraMessage}, video::VideoConfig};
use crossbeam_channel::{Sender, Receiver};
use image::imageops::FilterType;
use nokhwa::{Camera, pixel_format::RgbFormat, utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType}};
use std::{sync::Arc, thread};

const MJPEG: &str = "MJPEG";
const YUYV: &str = "YUYV";
const NV12: &str = "NV12";
const GRAY: &str = "GRAY";

pub fn start_thread(tx: Sender<CameraMessage>, cmd_rx: Receiver<CameraCommand>) {
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

            let real_fps = camera.frame_rate();
            let _ = tx.send(CameraMessage::StreamStarted(cfg.width, cfg.height, real_fps));

            loop {
                if let Ok(CameraCommand::Retry) = cmd_rx.try_recv() {
                    break;
                }

                match camera.frame() {
                    Ok(frame) => {
                        let raw_data = frame.buffer().to_vec();
                        let raw_arc = Arc::new(raw_data);
                        let raw =  raw_arc.clone();
                        if let Ok(decoded) = frame.decode_image::<RgbFormat>() {
                            let preview_img = image::imageops::resize(&decoded, 854, 480, FilterType::Nearest);
                            let p_width = preview_img.width();
                            let p_height = preview_img.height();
                            let preview = preview_img.into_raw();
                            let _ = tx.send(CameraMessage::Frame {
                                raw,
                                preview,
                                p_width,
                                p_height
                            });
                        }
                    },
                    Err(_) => {
                        let _ = tx.send(CameraMessage::Error("Camera lost".to_string()));
                        break;
                    }
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