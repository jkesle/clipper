use crate::messages::video::VideoConfig;
use std::sync::Arc;
pub enum CameraMessage {
    Capabilities(Vec<VideoConfig>),
    Frame {
        raw: Arc<Vec<u8>>,
        preview: Vec<u8>,
        p_width: u32,
        p_height: u32
    },
    StreamStarted(u32, u32, u32),
    Error(String)
}

pub enum CameraCommand {
    StartStream(VideoConfig),
    Retry
}