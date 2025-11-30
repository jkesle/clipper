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
