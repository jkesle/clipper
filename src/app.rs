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

use std::path::PathBuf;

use crate::messages::{audio::{AudioDevice, AudioMessage}, camera::{CameraCommand, CameraMessage}, recorder::{ClipInfo, RecorderCommand, RecorderStatus}, video::VideoConfig};
use crate::recorder::types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, App, Frame};
use chrono::Local;
use egui_extras::install_image_loaders;

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
    playlist: Vec<ClipInfo>,
    last_error: Option<String>,
    final_file: Option<String>,
    dragged_item: Option<usize>,
}

impl ClipperApp {
    pub fn new(_cc: &eframe::CreationContext, camera_rx: Receiver<CameraMessage>, camera_tx: Sender<CameraCommand>, rec_tx: Sender<RecorderCommand>, rec_status: Receiver<RecorderStatus>, audio_rx: Receiver<AudioMessage>) -> Self {
        egui_extras::install_image_loaders(&_cc.egui_ctx);
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
            dragged_item: None,
            last_error: None
        }
    }
}

impl App for ClipperApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
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
                RecorderStatus::SegmentSaved(p) => self.playlist.push(p),
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
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !self.is_recording && !self.playlist.is_empty() {
            let file_choice = rfd::FileDialog::new()
                .add_filter("video", &["mp4"])
                .set_file_name("vid.mp4")
                .set_directory("~")
                .save_file();

            if let Some(path) = file_choice {
                let output_path_string = path.to_string_lossy().to_string();
                let clip_paths: Vec<std::path::PathBuf> = self.playlist.iter()
                    .map(|c| c.video_path.clone())
                    .collect();
                let _ = self.rec_tx.send(RecorderCommand::FinalizeVideo(clip_paths, output_path_string));
            }
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
        ui.horizontal(|ui| {
            if self.is_recording {
                ui.colored_label(egui::Color32::RED, "RECORDING");
            } else {
                ui.label("Idle");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !self.playlist.is_empty() && !self.is_recording {
                    if ui.button("Merge").clicked() {
                        let file_choice = rfd::FileDialog::new().add_filter("video", &["mp4"]).set_file_name("vid.mp4").set_directory(".").save_file();
                        if let Some(path) = file_choice {
                            let output_path_string = path.to_string_lossy().to_string();
                            let clip_paths : Vec<PathBuf> = self.playlist.iter().map(|c| c.video_path.clone()).collect();
                            let _ = self.rec_tx.send(RecorderCommand::FinalizeVideo(clip_paths, output_path_string));
                        }
                    }
                }
            });
        });

        ui.separator();
        let total_height = ui.available_height();
        let timeline_height = 150.0;
        let camera_height = total_height - timeline_height;
        let camera_rect = ui.allocate_ui(egui::vec2(ui.available_width(), camera_height), |ui| {
            if let Some(texture) = &self.texture {
                let size = texture.size_vec2();
                let aspect = size.x / size.y;
                let available_w = ui.available_width();
                let available_h = ui.available_height();

                let (w, h) = if available_w / aspect <= available_h {
                    (available_w, available_w / aspect)
                } else {
                    (available_h * aspect, available_h)
                };

                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::new(texture).fit_to_exact_size(egui::vec2(w, h)));
                });
            }  
        }).response.rect;

        if self.is_recording {
            ui.put(egui::Rect::from_min_size(camera_rect.min + egui::vec2(20.0, 20.0), egui::vec2(200.0, 50.0)),
    |ui: &mut egui::Ui| ui.heading(egui::RichText::new("REC").color(egui::Color32::RED).strong())
            );
        }

        ui.put(
            egui::Rect::from_min_size(camera_rect.left_bottom() + egui::vec2(20.0, -50.0), egui::vec2(300.0, 50.0)),
            |ui: &mut egui::Ui| ui.colored_label(egui::Color32::WHITE, format!("Clips: {}", self.playlist.len()))
        );
        
        if let Some(f) = &self.final_file {
            ui.put(
                egui::Rect::from_center_size(camera_rect.center(), egui::vec2(400.0, 50.0)),
                |ui: &mut egui::Ui| ui.colored_label(egui::Color32::GREEN, format!("Saved: {}", f))
            );
        }

        if let Some(e) = &self.last_error {
            ui.put(
                egui::Rect::from_min_size(camera_rect.left_bottom() + egui::vec2(20.0, -100.0), egui::vec2(400.0, 40.0)),
                |ui: &mut egui::Ui| ui.colored_label(egui::Color32::RED, format!("{}", e))
            );
        }

        ui.separator();
        ui.label("Timeline");
        egui::ScrollArea::horizontal().min_scrolled_height(120.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut move_from = None;
                let mut move_to = None;
                let mut delete_index: Option<usize> = None;
                for (index, clip) in self.playlist.iter().enumerate() {
                    let size = egui::vec2(120.0, 90.0);
                    let item_id = ui.make_persistent_id(index);
                    let is_being_dragged = self.dragged_item == Some(index);
                    let response = ui.group(|ui| {
                        ui.set_min_size(size);
                        let hover_state = ui.ui_contains_pointer();
                        let img_source = if hover_state {
                            format!("file://{}", clip.preview_path.to_string_lossy())
                        } else {
                            format!("file://{}", clip.thumb_path.to_string_lossy())
                        };

                        let img_resp = ui.add(egui::Image::new(img_source).fit_to_exact_size(size).rounding(4.0));
                        let rect = img_resp.rect;
                        ui.painter().text(
                            rect.min + egui::vec2(5.0, 5.0),
                            egui::Align2::LEFT_TOP,
                            format!("{}", index + 1),
                            egui::FontId::proportional(20.0),
                            egui::Color32::WHITE
                        );

                        if hover_state {
                            let delete_btn_rect = egui::Rect::from_min_size(rect.max - egui::vec2(25.0, 25.0), egui::vec2(20.0, 20.0));
                            if ui.put(delete_btn_rect, egui::Button::new("X").small()).clicked() {
                                delete_index = Some(index);
                            }
                        }
                    }).response;

                    let response = response.interact(egui::Sense::drag());
                    if response.drag_started() {
                        self.dragged_item = Some(index);
                    }

                    if is_being_dragged {
                        ui.painter().rect_stroke(response.rect, 2.0, egui::Stroke::new(2.0, egui::Color32::YELLOW), egui::StrokeKind::Middle);
                    }

                    if let Some(dragged_index) = self.dragged_item {
                        if dragged_index != index && response.hovered() {
                            move_from = Some(dragged_index);
                            move_to = Some(index)
                        }
                    }
                }

                if let (Some(from), Some(to)) = (move_from, move_to) {
                    let item = self.playlist.remove(from);
                    self.playlist.insert(to, item);
                    self.dragged_item = Some(to);
                }

                if ui.input(|i| i.pointer.any_released()) {
                    self.dragged_item = None;
                }
            })
        });
    }
}