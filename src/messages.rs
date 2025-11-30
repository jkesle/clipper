use std::{fmt, sync::Arc, path::PathBuf};
use crate::recorder::types::{EncoderPreset, EncodingQuality, EncodingSpeed};

#[derive(Debug, Clone, PartialEq)]
pub struct VideoConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub fmt: String
}

impl fmt::Display for VideoConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}@{}fps ({})", self.width, self.height, self.fps, self.fmt)
    }
}

pub enum CameraMessage {
    Capabilities(Vec<VideoConfig>),
    Frame {
        raw: Arc<Vec<u8>>,
        preview: Vec<u8>,
        p_width: u32,
        p_height: u32
    },
    Error(String)
}

pub enum CameraCommand {
    StartStream(VideoConfig)
}

pub enum RecorderCommand {
    StartSegment,
    WriteFrame(Arc<Vec<u8>>),
    EndSegment,
    Undo,
    UpdateConfig { width: u32, height: u32, fps: u32, format: String, encoder: EncoderPreset, quality: EncodingQuality, speed: EncodingSpeed },
    FinalizeVideo(String)
}

pub enum RecorderStatus {
    SegmentSaved(PathBuf),
    SegmentDeleted,
    VideoFinalized(PathBuf)
}