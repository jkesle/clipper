
use crossbeam_channel::Sender;

#[derive(Clone, Debug, PartialEq)]
pub struct AudioDevice {
    pub name: String,
    pub index: usize
}

pub enum AudioMessage {
    DeviceList(Vec<AudioDevice>),
    Error(String)
}

pub enum AudioCommand {
    SelectDevice(usize),
    StartRecording(String),
    StopRecording(Sender<()>)
}