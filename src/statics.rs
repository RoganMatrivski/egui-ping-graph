use std::sync::OnceLock;
use time::{Instant, OffsetDateTime};

pub static DATETIME_START: OnceLock<OffsetDateTime> = OnceLock::new();
pub static I_START: OnceLock<Instant> = OnceLock::new();
