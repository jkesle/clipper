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

use std::fmt;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum EncoderPreset {
    CPU, 
    NVIDIA,
    AMD,
    INTEL
}

impl fmt::Display for EncoderPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncoderPreset::CPU => write!(f, "CPU (Universal / libx264)"),
            EncoderPreset::NVIDIA => write!(f, "NVIDIA (NVENC)"),
            EncoderPreset::AMD => write!(f, "AMD (AMF)"),
            EncoderPreset::INTEL => write!(f, "Intel (QuickSync)")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum EncodingQuality {
    High, 
    Med,
    Low
}

impl fmt::Display for EncodingQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingQuality::High => write!(f, "HQ (Larger file)"),
            EncodingQuality::Med => write!(f, "SQ (Standard)"),
            EncodingQuality::Low => write!(f, "LQ (Smallest file)")
        }        
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum EncodingSpeed {
    Fastest, 
    Balanced,
    Compact
}

impl fmt::Display for EncodingSpeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EncodingSpeed::Fastest => write!(f, "Fastest (Low CPU)"),
            EncodingSpeed::Balanced => write!(f, "Balanced"),
            EncodingSpeed::Compact => write!(f, "Compact (High CPU, Smaller file)")
        }
    }
}