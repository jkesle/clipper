use eframe::epaint::tessellator::path;

use crate::recorder::types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use std::{path::PathBuf, sync::Arc, time::Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct ClipInfo {
    pub video_path: PathBuf,
    pub thumb_path: PathBuf,
    pub preview_path: PathBuf,
    pub duration: f64
}

pub enum RecorderCommand {
    StartSegment,
    WriteFrame(Arc<Vec<u8>>, Instant),
    EndSegment,
    Undo,
    UpdateConfig { width: u32, height: u32, fps: u32, format: String, encoder: EncoderPreset, quality: EncodingQuality, speed: EncodingSpeed },
    SetAudioDevice(usize),
    FinalizeVideo(Vec<PathBuf>, String)
}

pub enum RecorderStatus {
    SegmentSaved(ClipInfo),
    SegmentDeleted,
    VideoFinalized(PathBuf),
    Error(String)
}