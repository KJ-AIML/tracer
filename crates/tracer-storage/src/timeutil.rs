//! RFC 3339 UTC timestamps for durable records.

use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

/// Current UTC timestamp as RFC 3339 string (`...Z` when offset is zero).
pub fn now_rfc3339() -> String {
    let now = OffsetDateTime::now_utc();
    now.format(&Rfc3339)
        .unwrap_or_else(|_| now.unix_timestamp().to_string())
}
