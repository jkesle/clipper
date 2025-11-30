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

use crate::messages::{CameraCommand, CameraMessage, RecorderCommand, RecorderStatus, VideoConfig};
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
    state: AppState,
    available_formats: Vec<VideoConfig>,
    selected_format: Option<VideoConfig>,
    selected_encoder: EncoderPreset,
    selected_quality: EncodingQuality,
    selected_speed: EncodingSpeed,
    texture: Option<egui::TextureHandle>,
    is_recording: bool,
    playlist: Vec<String>,
    last_final_file: Option<String>
}

impl ClipperApp {
    pub fn new(_cc: &eframe::CreationContext, camera_rx: Receiver<CameraMessage>, camera_tx: Sender<CameraCommand>, rec_tx: Sender<RecorderCommand>, rec_status: Receiver<RecorderStatus>) -> Self {
        Self {
            camera_rx,
            camera_tx,
            rec_tx,
            rec_status,
            state: AppState::Loading,
            available_formats: Vec::new(),
            selected_format: None,
            selected_encoder: EncoderPreset::CPU,
            selected_quality: EncodingQuality::Med,
            selected_speed: EncodingSpeed::Balanced,
            texture: None,
            is_recording: false,
            playlist: Vec::new(),
            last_final_file: None
        }
    }
}

impl App for ClipperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        while let Ok(msg) = self.camera_rx.try_recv() {
            match msg {
                CameraMessage::Capabilities(caps) => {
                    self.available_formats = caps;
                    if let Some(first) = self.available_formats.first() {
                        self.selected_format = Some(first.clone());
                    }
                    
                    self.state = AppState::Configuring;
                },
                CameraMessage::Frame { raw, preview, p_width, p_height} => {
                    let image = egui::ColorImage::from_rgb([p_width as usize, p_height as usize], &preview);
                    self.texture = Some(ctx.load_texture("cam", image, Default::default()));
                    if self.is_recording {
                        let _ = self.rec_tx.send(RecorderCommand::WriteFrame(raw));
                    }
                }

                CameraMessage::Error(e) => { eprintln!("Camera error: {}", e); }
            }
        }

        while let Ok(status) = self.rec_status.try_recv() {
            match status {
                RecorderStatus::SegmentSaved(p) => self.playlist.push(p.to_string_lossy().into()),
                RecorderStatus::SegmentDeleted => { self.playlist.pop(); },
                RecorderStatus::VideoFinalized(p) => {
                    self.playlist.clear();
                    self.last_final_file = Some(p.to_string_lossy().to_string());
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state {
                AppState::Loading => {
                    ui.centered_and_justified(|ui| {
                        ui.vertical(|ui| {
                            ui.spinner();
                            ui.label("Querying camera capabilities...");
                        });
                    });
                },
                AppState::Configuring => {
                    ui.centered_and_justified(|ui| {
                        ui.vertical(|ui| {
                            self.show_config(ui);
                        });
                    });
                },

                AppState::Running => self.show_running(ctx, ui)
            }
        });

        ctx.request_repaint();
    }
}

impl ClipperApp {
    fn show_config(&mut self, ui: &mut egui::Ui) {
        ui.heading("Configure");
        ui.separator();
        egui::Grid::new("cfg").show(ui, |ui| {
            ui.label("Format:");
            if let Some(sel) = &mut self.selected_format {
                egui::ComboBox::from_id_salt("fmt").selected_text(sel.to_string()).show_ui(ui, |ui| {
                    for config in &self.available_formats { ui.selectable_value(sel, config.clone(), config.to_string()); }
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

        if ui.button("Confirm").clicked() {
            if let Some(cfg) = &self.selected_format {
                let _ = self.camera_tx.send(CameraCommand::StartStream(cfg.clone()));
                let _ = self.rec_tx.send(RecorderCommand::UpdateConfig {
                    width: cfg.width, height: cfg.height, fps: cfg.fps, format: cfg.fmt.clone(), encoder: self.selected_encoder, quality: self.selected_quality, speed: self.selected_speed
                });
                self.state = AppState::Running;
            }
        }
    }

    fn show_running(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if let Some(tex) = &self.texture {
            ui.add(egui::Image::new(tex).fit_to_exact_size(ui.available_size()));
        }

        if self.is_recording { 
            ui.put(
                egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(200.0, 50.0)),
                |ui: &mut egui::Ui| ui.heading(egui::RichText::new("RECORDING").color(egui::Color32::RED).strong())
            );
        }

        ui.put(
            egui::Rect::from_min_size(egui::pos2(20.0, ui.available_height() - 50.0), egui::vec2(300.0, 50.0)),
            |ui: &mut egui::Ui| ui.colored_label(egui::Color32::WHITE, format!("Clips: {}", self.playlist.len()))
        );

        if let Some(file) = &self.last_final_file {
            ui.centered_and_justified(|ui| {
                ui.colored_label(egui::Color32::GREEN, format!("Saved: {}", file));
            });
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !self.is_recording {
            self.is_recording = true;
            self.last_final_file = None;
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
            let _ = self.rec_tx.send(RecorderCommand::FinalizeVideo(format!("output_{}.mp4", Local::now().format("%Y-%m-%d_%H%M%S%.3f").to_string())));
        }
    }
}