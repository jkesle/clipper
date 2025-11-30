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