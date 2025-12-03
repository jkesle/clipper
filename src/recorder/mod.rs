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

pub mod types;
mod ffmpeg;

use crate::messages::{audio::AudioCommand, recorder::{RecorderCommand, RecorderStatus}};
use types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use crossbeam_channel::{Receiver, Sender};
use std::{fs::{self, File}, io::Write, path::PathBuf, process::{Child, Command, Stdio}, thread};

pub fn start_thread(cmd_rx: Receiver<RecorderCommand>, status_tx: Sender<RecorderStatus>, aud_tx: Sender<AudioCommand>) {
    thread::spawn(move || {
        let mut video_process: Option<Child> = None;
        let mut segments: Vec<PathBuf> = Vec::new();
        let mut counter = 0;
        let mut width = 640;
        let mut height = 480;
        let mut fps = 30;
        let mut format = String::from("MJPEG");
        let mut encoder = EncoderPreset::CPU;
        let mut quality = EncodingQuality::Med;
        let mut speed = EncodingSpeed::Balanced;
        let temp_vid: &str = "tmp_vid.mp4";
        let temp_aud: &str = "tmp_aud.mp4";

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                RecorderCommand::UpdateConfig {width: w, height: h, fps: f, format: fmt, encoder: enc, quality: qty, speed: spd } => {
                    width = w; height = h; fps = f; format = fmt; encoder = enc; quality = qty; speed = spd;
                    println!("Recorder config updated: {}x{}@{} fps ({})", width, height, fps, format);
                },
                RecorderCommand::SetAudioDevice(index) => {
                    if let Err(e) = aud_tx.send(AudioCommand::SelectDevice(index)) {
                        let _ = status_tx.send(RecorderStatus::Error(format!("Audio thread lost: {}", e)));
                    }
                },
                RecorderCommand::StartSegment => {
                    counter += 1;
                    let _ = aud_tx.send(AudioCommand::StartRecording(String::from(temp_aud)));
                    let args = ffmpeg::build_cmd(width, height, fps, &format, encoder, quality, speed, temp_vid);
                    let child = Command::new("ffmpeg").args(&args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit()).spawn();
                    match child {
                        Ok(c) => video_process = Some(c),
                        Err(e) => { let _ = status_tx.send(RecorderStatus::Error(format!("Failed to spawn ffmpeg: {}", e))); }
                    }
                },
                RecorderCommand::WriteFrame(data) => {
                    if let Some(proc) = &mut video_process {
                        if let Some(stdin) = &mut proc.stdin {
                            let _ = stdin.write_all(&data);
                        }
                    }
                },
                RecorderCommand::EndSegment => {
                    if let Some(mut proc) = video_process.take() {
                        if let Err(e) = proc.wait() {
                            eprintln!("Video process wait error: {}", e);
                        }
                    }

                    let (ack_tx, ack_rx) = crossbeam_channel::bounded(1);
                    if let Err(e) = aud_tx.send(AudioCommand::StopRecording(ack_tx)) {
                        eprintln!("Audio thread unavailable: {}", e);
                    } else if let Err(_) = ack_rx.recv() {
                        eprintln!("Audio thread disconnected unexpectedly during flush");
                    }

                    if !std::path::Path::new(temp_vid).exists() || !std::path::Path::new(temp_aud).exists() {
                        let _ = status_tx.send(RecorderStatus::Error("Temp files missing, recording failed".into()));
                        let _ = fs::remove_file(temp_vid);
                        let _ = fs::remove_file(temp_aud);
                        continue;
                    }

                    let finfile = format!("clip_{:03}.mp4", counter);
                    println!("Merging to {}", finfile);

                    let merge = Command::new("ffmpeg").args(&[
                        "-i", temp_vid,
                        "-i", temp_aud,
                        "-c:v", "copy",
                        "-c:a", "aac",
                        "-y", &finfile
                    ]).stdout(Stdio::null()).stderr(Stdio::inherit()).status();
                    match merge {
                        Ok(s) if s.success() => {
                            segments.push(PathBuf::from(&finfile));
                            let _ = status_tx.send(RecorderStatus::SegmentSaved(PathBuf::from(&finfile)));
                            let _ = fs::remove_file(temp_vid);
                            let _ = fs::remove_file(temp_aud);
                        },
                        Ok(_) | Err(_) => { let _ = status_tx.send(RecorderStatus::Error("Merge failed".into())); }
                    }
                },
                RecorderCommand::Undo => {
                    if let Some(path) = segments.pop() {
                        if let Err(e) = fs::remove_file(&path) {
                            eprintln!("Failed to delete file: {}", e);
                        }

                        let _ = status_tx.send(RecorderStatus::SegmentDeleted);
                    }
                },
                RecorderCommand::FinalizeVideo(outfile) => {
                    if segments.is_empty() { continue; }
                    let list_file = "concat_list.txt";
                    if let Ok(mut f) = fs::File::create(list_file) {
                        for seg in &segments {
                            let _ = writeln!(f, "file '{}'", seg.to_string_lossy());
                        }
                    }

                    let status = Command::new("ffmpeg").args(&[
                        "-f", "concat", 
                        "-safe", "0", 
                        "-i", list_file, 
                        "-c", "copy", 
                        "-y", &outfile
                        ]).stdout(Stdio::null()).stderr(Stdio::inherit()).status();
                    match status {
                        Ok(s) if s.success() => {
                            let _ = status_tx.send(RecorderStatus::VideoFinalized(PathBuf::from(&outfile)));
                            let _ = fs::remove_file(list_file);
                            for seg in &segments { let _ = fs::remove_file(seg); }
                            segments.clear();
                            counter = 0;
                        },
                        _ => { let _ = status_tx.send(RecorderStatus::Error("Final concat fialed".into())); }
                    }
                }
            }
        }
    });
}