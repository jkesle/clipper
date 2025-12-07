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

use crate::messages::{audio::{AudioDevice, AudioMessage}, camera::{CameraCommand, CameraMessage}, recorder::{RecorderCommand, RecorderStatus}, video::VideoConfig};
use crate::recorder::types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, App, Frame};
use chrono::Local;

#[derive(PartialEq)]
enum AppState {
    Loading,
    Configuring,
    Running
}

pub struct ClipperApp {
    camera_rx: Receiver<CameraMessage>,
    camera_tx: Sender<CameraCommand>,
    rec_tx: Sender<RecorderCommand>,
    rec_status: Receiver<RecorderStatus>,
    audio_rx: Receiver<AudioMessage>,
    state: AppState,
    video_configs: Vec<VideoConfig>,
    selected_video_config: Option<VideoConfig>,
    audio_devices: Vec<AudioDevice>,
    selected_audio_device: Option<AudioDevice>,
    selected_encoder: EncoderPreset,
    selected_quality: EncodingQuality,
    selected_speed: EncodingSpeed,
    texture: Option<egui::TextureHandle>,
    is_recording: bool,
    playlist: Vec<String>,
    last_error: Option<String>,
    final_file: Option<String>
}

impl ClipperApp {
    pub fn new(_cc: &eframe::CreationContext, camera_rx: Receiver<CameraMessage>, camera_tx: Sender<CameraCommand>, rec_tx: Sender<RecorderCommand>, rec_status: Receiver<RecorderStatus>, audio_rx: Receiver<AudioMessage>) -> Self {
        Self {
            camera_rx,
            camera_tx,
            rec_tx,
            rec_status,
            audio_rx,
            state: AppState::Loading,
            video_configs: Vec::new(),
            selected_video_config: None,
            audio_devices: Vec::new(),
            selected_audio_device: None,
            selected_encoder: EncoderPreset::CPU,
            selected_quality: EncodingQuality::Med,
            selected_speed: EncodingSpeed::Balanced,
            texture: None,
            is_recording: false,
            playlist: Vec::new(),
            final_file: None,
            last_error: None
        }
    }
}

impl App for ClipperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Messages
        while let Ok(msg) = self.camera_rx.try_recv() {
            match msg {
                CameraMessage::Capabilities(c) => { self.video_configs = c; self.selected_video_config = self.video_configs.first().cloned(); self.state = AppState::Configuring; },
                CameraMessage::StreamStarted(w, h, fps) => {
                    if let Some(cfg) = &self.selected_video_config {
                        let _ = self.rec_tx.send(RecorderCommand::UpdateConfig {
                            width: w, height: h, fps, format: cfg.fmt.clone(),
                            encoder: self.selected_encoder, quality: self.selected_quality, speed: self.selected_speed
                        });
                    }
                },
                CameraMessage::Frame { raw: _, preview, p_width, p_height } => {
                    let img = egui::ColorImage::from_rgb([p_width as usize, p_height as usize], &preview);
                    self.texture = Some(ctx.load_texture("cam", img, Default::default()));
                },
                CameraMessage::Error(e) => self.last_error = Some(format!("Cam: {}", e)),
            }
        }

        while let Ok(msg) = self.audio_rx.try_recv() {
            match msg {
                AudioMessage::DeviceList(l) => { self.audio_devices = l; self.selected_audio_device = self.audio_devices.first().cloned(); },
                AudioMessage::Error(e) => self.last_error = Some(format!("Audio: {}", e)),
            }
        }

        while let Ok(stat) = self.rec_status.try_recv() {
            match stat {
                RecorderStatus::SegmentSaved(p) => self.playlist.push(p.to_string_lossy().to_string()),
                RecorderStatus::SegmentDeleted => { self.playlist.pop(); },
                RecorderStatus::VideoFinalized(p) => { self.playlist.clear(); self.final_file = Some(p.to_string_lossy().to_string()); },
                RecorderStatus::Error(e) => self.last_error = Some(format!("Rec: {}", e)),
            }
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !self.is_recording {
            self.is_recording = true; self.final_file = None; self.last_error = None;
            let _ = self.rec_tx.send(RecorderCommand::StartSegment);
        }
        if ctx.input(|i| i.key_released(egui::Key::Space)) && self.is_recording {
            self.is_recording = false;
            let _ = self.rec_tx.send(RecorderCommand::EndSegment);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Backspace)) && !self.is_recording {
             let _ = self.rec_tx.send(RecorderCommand::Undo);
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.is_recording {
             let _ = self.rec_tx.send(RecorderCommand::FinalizeVideo("output.mp4".to_string()));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state {
                AppState::Loading => {
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            if let Some(err) = &self.last_error {
                                ui.heading(egui::RichText::new("Camera initialization failed").color(egui::Color32::RED));
                                ui.label(err);
                                ui.add_space(10.0);
                                if ui.button("Retry").clicked() {
                                    self.last_error = None;
                                }
                            } else {
                                ui.spinner();
                                ui.label("Querying Camera...");
                            }
                        });
                    });
                },
                AppState::Configuring => self.show_config(ui),
                AppState::Running => self.show_running(ui),
            }
        });
        
        ctx.request_repaint();
    }
}

impl ClipperApp {
    fn show_config(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configure");
        ui.separator();
        egui::Grid::new("cfg_grid").show(ui, |ui| {
            ui.label("Video:");
            if let Some(sel) = &mut self.selected_video_config {
                egui::ComboBox::from_id_salt("vid").selected_text(sel.to_string()).show_ui(ui, |ui| {
                    for config in &self.video_configs { ui.selectable_value(sel, config.clone(), config.to_string()); }
                });
            }
            ui.end_row();

            ui.label("Audio:");
            if let Some(sel) = &mut self.selected_audio_device {
                egui::ComboBox::from_id_salt("aud").selected_text(&sel.name).show_ui(ui, |ui| {
                    for device in &self.audio_devices {
                        if ui.selectable_value(sel, device.clone(), &device.name).clicked() {
                            let _ = self.rec_tx.send(RecorderCommand::SetAudioDevice(device.index));
                        }
                    }
                });
            }
            ui.end_row();

            ui.label("Encoder:");
            egui::ComboBox::from_id_salt("enc").selected_text(self.selected_encoder.to_string()).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_encoder, EncoderPreset::CPU, "CPU");
                ui.selectable_value(&mut self.selected_encoder, EncoderPreset::NVIDIA, "NVIDIA");
                ui.selectable_value(&mut self.selected_encoder, EncoderPreset::AMD, "AMD");
                ui.selectable_value(&mut self.selected_encoder, EncoderPreset::INTEL, "Intel");
            });
            ui.end_row();

            ui.label("Encoding Quality:");
            egui::ComboBox::from_id_salt("qty").selected_text(self.selected_quality.to_string()).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_quality, EncodingQuality::High, format!("{}", EncodingQuality::High));
                ui.selectable_value(&mut self.selected_quality, EncodingQuality::Med, format!("{}", EncodingQuality::Med));
                ui.selectable_value(&mut self.selected_quality, EncodingQuality::Low, format!("{}", EncodingQuality::Low));
            });
            ui.end_row();

            ui.label("Encoding Speed:");
            egui::ComboBox::from_id_salt("spd").selected_text(self.selected_speed.to_string()).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.selected_speed, EncodingSpeed::Fastest, format!("{}", EncodingSpeed::Fastest));
                ui.selectable_value(&mut self.selected_speed, EncodingSpeed::Balanced, format!("{}", EncodingSpeed::Balanced));
                ui.selectable_value(&mut self.selected_speed, EncodingSpeed::Compact, format!("{}", EncodingSpeed::Compact));
            });
            ui.end_row();
        });

        ui.add_space(20.0);
        if ui.button("Confirm").clicked() {
            if let Some(cfg) = &self.selected_video_config {
                let _ = self.camera_tx.send(CameraCommand::StartStream(cfg.clone()));
                let _ = self.rec_tx.send(RecorderCommand::UpdateConfig {
                    width: cfg.width, height: cfg.height, fps: cfg.fps, format: cfg.fmt.clone(), encoder: self.selected_encoder, quality: self.selected_quality, speed: self.selected_speed
                });
                self.state = AppState::Running;
            }
        }
    }

    fn show_running(&mut self, ui: &mut egui::Ui) {
        if let Some(tex) = &self.texture {
            ui.add(egui::Image::new(tex).fit_to_exact_size(ui.available_size()));
        }

        if self.is_recording {
            ui.put(egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(200.0, 50.0)),
                   |ui: &mut egui::Ui| ui.heading(egui::RichText::new("REC").color(egui::Color32::RED).strong()));
        }

        ui.put(egui::Rect::from_min_size(egui::pos2(20.0, ui.available_height() - 50.0), egui::vec2(300.0, 50.0)),
               |ui: &mut egui::Ui| ui.colored_label(egui::Color32::WHITE, format!("Clips: {}", self.playlist.len())));
        
        if let Some(f) = &self.final_file {
            ui.centered_and_justified(|ui| ui.colored_label(egui::Color32::GREEN, format!("Saved: {}", f)));
        }

        if let Some(e) = &self.last_error {
            ui.put(egui::Rect::from_min_size(egui::pos2(20.0, ui.available_height() - 100.0), egui::vec2(400.0, 40.0)),
               |ui: &mut egui::Ui| ui.colored_label(egui::Color32::RED, format!("{}", e)));
        }
    }
}