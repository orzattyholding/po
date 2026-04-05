//! Logical channel multiplexing for PO sessions.
//!
//! Channels allow multiple independent data streams over a single PO connection.
//! Each channel has a numeric ID (u32) that maps to a specific purpose.

/// Well-known channel IDs.
pub mod channels {
    /// Control channel (handshake, ping, close).
    pub const CONTROL: u32 = 0;
    /// Default data channel.
    pub const DEFAULT: u32 = 1;
    /// File transfer channel.
    pub const FILE_TRANSFER: u32 = 2;
    /// First user-defined channel ID.
    pub const USER_START: u32 = 100;
}
