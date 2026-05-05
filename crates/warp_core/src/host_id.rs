use std::fmt;

/// Opaque identifier for a remote host.
///
/// Retained for restored remote-session metadata and host-scoped models.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostId(String);

impl HostId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
