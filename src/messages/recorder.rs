use crate::recorder::types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use std::{sync::Arc, path::PathBuf};
pub enum RecorderCommand {
    StartSegment,
    WriteFrame(Arc<Vec<u8>>),
    EndSegment,
    Undo,
    UpdateConfig { width: u32, height: u32, fps: u32, format: String, encoder: EncoderPreset, quality: EncodingQuality, speed: EncodingSpeed },
    SetAudioDevice(usize),
    FinalizeVideo(String)
}

pub enum RecorderStatus {
    SegmentSaved(PathBuf),
    SegmentDeleted,
    VideoFinalized(PathBuf),
    Error(String)
}