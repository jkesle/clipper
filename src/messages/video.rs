use std::fmt;

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