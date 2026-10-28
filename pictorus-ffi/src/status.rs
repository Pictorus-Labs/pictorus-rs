//! Status codes returned across the C boundary.

pub type pictorus_status_t = i32;

pub const PICTORUS_OK: pictorus_status_t = 0;
/// No data available. Not an error.
pub const PICTORUS_EMPTY: pictorus_status_t = 1;
pub const PICTORUS_ERR_PERIPHERAL: pictorus_status_t = -1;
pub const PICTORUS_ERR_INVALID_ARG: pictorus_status_t = -2;
pub const PICTORUS_ERR_ALREADY_INIT: pictorus_status_t = -3;
