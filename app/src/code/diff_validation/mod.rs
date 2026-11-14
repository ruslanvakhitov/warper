//! Diff data used by the code editor and Git diff viewer.

use std::{fmt, ops::Range};

/// Visual representation of a single diff hunk.
#[derive(Clone, PartialEq, Eq)]
pub struct DiffDelta {
    pub replacement_line_range: Range<usize>,
    pub insertion: String,
}

impl fmt::Debug for DiffDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if cfg!(debug_assertions) {
            write!(
                f,
                "DiffDelta {{\nreplacement_line_range: {:?},",
                &self.replacement_line_range
            )?;
            f.write_str("\n--insertion--\n")?;
            f.write_str(&self.insertion)?;
            f.write_str("\n}")
        } else {
            Ok(())
        }
    }
}
