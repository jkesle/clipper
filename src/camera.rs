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

use crate::messages::{CameraCommand, CameraMessage, VideoConfig};
use crossbeam_channel::{Sender, Receiver};
use image::imageops::FilterType;
use nokhwa::{Camera, pixel_format::RgbFormat, utils::{CameraFormat, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType}};
use std::{sync::Arc, thread};

pub fn start_camera_thread(tx: Sender<CameraMessage>, cmd_rx: Receiver<CameraCommand>) {
    thread::spawn(move || {
        let index: CameraIndex = CameraIndex::Index(0);
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = match Camera::new(index.clone(), requested) {
            Ok(c) => c,
            Err(e) => {
                let _= tx.send(CameraMessage::Error(format!("Camera initialization failed: {}", e)));
                return;
            }
        };

        match camera.compatible_camera_formats() {
            Ok(formats) => {
                let mut configs = Vec::new();
                for fmt in formats {
                    let config = VideoConfig {
                        width: fmt.resolution().width(),
                        height: fmt.resolution().height(),
                        fps: fmt.frame_rate(),
                        fmt: fmt.format().to_string()
                    };

                    if !configs.contains(&config) { configs.push(config); }
                }

                configs.sort_by(|a, b| {
                    b.width.cmp(&a.width).then(b.fps.cmp(&a.fps))
                });

                let _ = tx.send(CameraMessage::Capabilities(configs));
            }
            Err(e) => {
                let _ = tx.send(CameraMessage::Error(format!("Query caps failed: {}", e)));
                return;
            }
        }

        drop(camera);

        let selected_config = match cmd_rx.recv() {
            Ok(CameraCommand::StartStream(config)) => config,
            _ => return
        };

        let frame_fmt = match selected_config.fmt.as_str() {
            "MJPEG" => FrameFormat::MJPEG,
            "YUYV" => FrameFormat::YUYV,
            "NV12" => FrameFormat::NV12,
            "GRAY" => FrameFormat::GRAY,
            _ => FrameFormat::MJPEG
        };

        let exact_fmt = CameraFormat::new_from(
            selected_config.width,
            selected_config.height,
            frame_fmt,
            selected_config.fps
        );

        let req = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Exact(exact_fmt));

        let mut camera = match Camera::new(index, req) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(CameraMessage::Error(format!("Reinit failed: {}", e)));
                return;
            }
        };

        if let Err(e) = camera.open_stream() {
            let _ = tx.send(CameraMessage::Error(format!("Stream open failure: {}", e)));
            return;
        }

        println!("Camera started: {}", selected_config);

        loop {
            if let Ok(frame) = camera.frame() {
                let raw_data = frame.buffer().to_vec();
                let raw_arc = Arc::new(raw_data);
                let raw_for_rec =  raw_arc.clone();
                if let Ok(decoded) = frame.decode_image::<RgbFormat>() {
                    let preview = image::imageops::resize(&decoded, 854, 480, FilterType::Nearest);
                    let p_width = preview.width();
                    let p_height = preview.height();
                    let p_data = preview.into_raw();
                    let _ = tx.send(CameraMessage::Frame {
                        raw: raw_for_rec,
                        preview: p_data,
                        p_width,
                        p_height
                    });
                }
            }
        }
    });
}