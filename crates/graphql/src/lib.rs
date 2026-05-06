mod api;
pub use api::*;

pub mod managed_secrets;

pub mod scalars;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Id(String);

impl Id {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}
