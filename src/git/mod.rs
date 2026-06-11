mod status;

pub use status::{ChangeKind, ChangedFile, GitError, GitStatusSummary, parse_status, read_status};
