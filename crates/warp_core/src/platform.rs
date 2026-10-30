use line_ending::LineEnding;

#[derive(Debug, Clone)]
pub enum SessionPlatform {
    MSYS2,
    WSL,
    Native,
}

impl SessionPlatform {
    #[allow(clippy::disallowed_methods)]
    pub fn default_line_ending(&self) -> LineEnding {
        match self {
            SessionPlatform::MSYS2 | SessionPlatform::WSL => LineEnding::LF,
            SessionPlatform::Native => LineEnding::from_current_platform(),
        }
    }
}
