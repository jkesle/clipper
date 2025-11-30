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

use super::types::{EncoderPreset, EncodingQuality, EncodingSpeed};

pub fn build_cmd(width: u32, height: u32, fps: u32, format: &str, encoder: EncoderPreset, quality: EncodingQuality, speed: EncodingSpeed, filename: &str) -> Vec<String> {
    let f = String::from("-f");
    let framerate = String::from("-framerate");
    let fpstr = fps.to_string();
    let i = String::from("-i");
    let dash = String::from("-");

    let mut args = match format {
        "MJPEG" => vec![
            f, String::from("mjpeg"),
            framerate, fpstr,
            i, dash
        ],
        _ => vec![
            f, String::from("rawvideo"),
            String::from("-pixel_format"), if format == "YUYV" { "yuyv422" } else { "rgb24" }.to_string(),
            String::from("-video_size"), format!("{}x{}", width, height),
            framerate, fpstr,
            i, dash
        ]
    };

    let enc_args = match encoder {
        EncoderPreset::CPU => {
            let preset = match speed {
                EncodingSpeed::Fastest => "ultrafast",
                EncodingSpeed::Balanced => "veryfast",
                EncodingSpeed::Compact => "medium"
            };

            let crf = match quality {
                EncodingQuality::High => "18",
                EncodingQuality::Med => "23",
                EncodingQuality::Low => "28"
            };

            vec!["-c:v", "libx264", "-pix_fmt", "yuv420p",
                "-preset", preset, "-crf", crf, "-tune", "zerolatency"]
        },
        
        EncoderPreset::NVIDIA => {
            let preset = match speed {
                EncodingSpeed::Fastest => "p1",
                EncodingSpeed::Balanced => "p4",
                EncodingSpeed::Compact => "p7"
            };

            let cq = match quality {
                EncodingQuality::High => "19",
                EncodingQuality::Med => "23",
                EncodingQuality::Low => "28"
            };

            vec!["-c:v", "h264_nvenc", "-pix_fmt", "yuv420p",
                "-preset", preset, "-rc:v", "vbr", "-cq", cq]
        },

        EncoderPreset::AMD => vec!["-c:v", "h264_amf", "-usage", "transcoding"],
        EncoderPreset::INTEL => vec!["-c:v", "h264_qsv", "-preset", "medium"]
    };

    for arg in enc_args { args.push(arg.to_string()); }
    args.push(String::from("-y"));
    args.push(filename.to_string());
    args
}