pub mod types;
mod ffmpeg;

use crate::messages::{RecorderCommand, RecorderStatus};
use types::{EncoderPreset, EncodingQuality, EncodingSpeed};
use crossbeam_channel::{Receiver, Sender};
use std::{fs::{self, File}, io::Write, path::PathBuf, process::{Child, Command, Stdio}, thread};

pub fn start_thread(cmd_rx: Receiver<RecorderCommand>, status_tx: Sender<RecorderStatus>) {
    thread::spawn(move || {
        let mut process: Option<Child> = None;
        let mut segments: Vec<PathBuf> = Vec::new();
        let mut counter = 0;
        let mut width = 640;
        let mut height = 480;
        let mut fps = 30;
        let mut format = String::from("MJPEG");
        let mut encoder = EncoderPreset::CPU;
        let mut quality = EncodingQuality::Med;
        let mut speed = EncodingSpeed::Balanced;

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                RecorderCommand::UpdateConfig {width: w, height: h, fps: f, format: fmt, encoder: enc, quality: qty, speed: spd } => {
                    width = w; height = h; fps = f; format = fmt; encoder = enc; quality = qty; speed = spd;
                },
                RecorderCommand::StartSegment => {
                    counter += 1;
                    let filename = format!("clip_{:03}.mp4", counter);
                    let args = ffmpeg::build_cmd(width, height, fps, &format, encoder, quality, speed, &filename);
                    println!("REC: fmmpeg {}", args.join(" "));
                    let child = Command::new("ffmpeg").args(&args).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::inherit()).spawn();
                    match child {
                        Ok(c) => process = Some(c),
                        Err(e) => eprintln!("FFmpeg error: {}", e)
                    }

                    segments.push(PathBuf::from(&filename));
                },
                RecorderCommand::WriteFrame(data) => {
                    if let Some(proc) = &mut process {
                        if let Some(stdin) = &mut proc.stdin {
                            let _ = stdin.write_all(&data);
                        }
                    }
                },
                RecorderCommand::EndSegment => {
                    if let Some(mut proc) = process.take() {
                        let _ = proc.wait();
                        if let Some(last) = segments.last() {
                            let _ = status_tx.send(RecorderStatus::SegmentSaved(last.clone()));
                        }
                    }
                },
                RecorderCommand::Undo => {
                    if let Some(path) = segments.pop() {
                        let _ = fs::remove_file(&path);
                        let _ = status_tx.send(RecorderStatus::SegmentDeleted);
                    }
                },
                RecorderCommand::FinalizeVideo(outfile) => {
                    if segments.is_empty() {
                        println!("No segments to merge.");
                        continue;
                    }
                    println!("Merging {} segments into {}", segments.len(), outfile);
                    let list_file_name = "concat_list.txt";
                    if let Ok(mut file) = File::create(list_file_name) {
                        for segment in &segments {
                            let _ = writeln!(file, "file '{}'", segment.to_string_lossy());
                        }
                    }

                    let status = Command::new("ffmpeg").args(&[
                        "-f", "concat",
                        "-safe", "0",
                        "-i", list_file_name,
                        "-c", "copy",
                        "-y", &outfile
                    ]).stdout(Stdio::null()).stderr(Stdio::inherit()).status();

                    if let Ok(s) = status {
                        if s.success() {
                            println!("Successfully merged clips");
                            let _ = status_tx.send(RecorderStatus::VideoFinalized(PathBuf::from(&outfile)));
                            let _ = fs::remove_file(list_file_name);
                            for seg in &segments {
                                let _ = fs::remove_file(seg);
                            }
                            segments.clear();
                            counter = 0;
                        }
                    }
                }
            }
        }
    });
}