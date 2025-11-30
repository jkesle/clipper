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

mod messages;
mod camera;
mod recorder;
mod app;

use crossbeam_channel::unbounded;
use eframe::NativeOptions;
fn main() -> eframe::Result<()> {
    let (cam_tx, cam_rx) = unbounded();
    let (cam_command_tx, cam_command_rx) = unbounded();
    let (rec_command_tx, rec_command_rx) = unbounded();
    let (rec_status_tx, rec_status_rx) = unbounded();
    camera::start_camera_thread(cam_tx, cam_command_rx);
    recorder::start_thread(rec_command_rx, rec_status_tx);
    let options = NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([800.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native("Clipper", options, Box::new(|cc| {
        Ok(Box::new(app::ClipperApp::new(cc, cam_rx, cam_command_tx, rec_command_tx, rec_status_rx)))
    }))
}
