//! Storage for bytes that cross the C boundary.
//!
//! Every byte buffer an adapter owns lives in a [`ByteBuffer`]. With an allocator it grows to
//! whatever an operation needs, as the platform adapters in the other crates do, and it keeps
//! its allocation between ticks so that a running model does not allocate.
//!
//! Supporting builds without an allocator is a change to this file only. Back `ByteBuffer` with
//! a `heapless::Vec<u8, N>`, take `N` as a const generic on the adapters, and source it from the
//! target's `buffer_size_bytes`. Operations that do not fit would then fail with a status code
//! of their own. The blocks that buffer data themselves (I2C, serial, SPI and CAN) still
//! allocate, so this is not the only thing a no-alloc build needs.

use alloc::vec::Vec;

use super::pictorus_peripheral_status_t;
use crate::status::{PICTORUS_EMPTY, PICTORUS_ERR_INVALID_ARG, PICTORUS_OK, pictorus_status_t};

/// The most bytes C is asked for in one receive from a peripheral with no natural message size,
/// such as a serial port or UDP socket. Bytes C does not hand over stay with C for a later tick.
pub const RECEIVE_BUFFER_BYTES: usize = pictorus_internal::protocols::BUFF_SIZE_BYTES;

/// The outcome of a receive operation. When it is not `Data` the buffer is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Received {
    Data,
    Empty,
    Failed,
}

#[derive(Default)]
pub(crate) struct ByteBuffer {
    data: Vec<u8>,
}

impl ByteBuffer {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub(crate) fn clear(&mut self) {
        self.data.clear();
    }

    /// Replaces the contents with `len` zeroed bytes for the caller to fill.
    pub(crate) fn prepare(&mut self, len: usize) -> &mut [u8] {
        self.data.clear();
        self.data.resize(len, 0);
        &mut self.data
    }

    /// Replaces the contents with `parts` joined together.
    pub(crate) fn set_concat(&mut self, parts: &[&[u8]]) {
        self.data.clear();
        for part in parts {
            self.data.extend_from_slice(part);
        }
    }

    /// Fills the buffer from a C operation that is offered up to `capacity` bytes and reports how
    /// much it wrote through its length argument.
    ///
    /// `PICTORUS_EMPTY`, or a successful call that wrote nothing, is "no data". A failure, or a
    /// length beyond what was offered, is recorded in `status`.
    pub(crate) fn receive(
        &mut self,
        status: &mut pictorus_peripheral_status_t,
        capacity: usize,
        op: impl FnOnce(*mut u8, usize, *mut usize) -> pictorus_status_t,
    ) -> Received {
        let buf = self.prepare(capacity);

        let mut len = 0usize;
        let result = op(buf.as_mut_ptr(), capacity, &mut len);

        if result == PICTORUS_EMPTY || (result == PICTORUS_OK && len == 0) {
            self.clear();
            return Received::Empty;
        }
        if !status.record(result) {
            self.clear();
            return Received::Failed;
        }
        if len > capacity {
            status.record(PICTORUS_ERR_INVALID_ARG);
            self.clear();
            return Received::Failed;
        }
        self.data.truncate(len);
        Received::Data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::status::PICTORUS_ERR_PERIPHERAL;

    #[test]
    fn prepare_replaces_contents_with_zeroes() {
        let mut buf = ByteBuffer::new();
        buf.prepare(3).copy_from_slice(&[1, 2, 3]);
        assert_eq!(buf.prepare(2), [0, 0]);
        assert_eq!(buf.as_slice(), [0, 0]);
    }

    #[test]
    fn concat_replaces_contents() {
        let mut buf = ByteBuffer::new();
        buf.set_concat(&[&[1], &[2, 3]]);
        assert_eq!(buf.as_slice(), [1, 2, 3]);
        buf.set_concat(&[&[9]]);
        assert_eq!(buf.as_slice(), [9]);
    }

    #[test]
    fn reuses_its_allocation() {
        let mut buf = ByteBuffer::new();
        buf.prepare(64);
        let before = buf.data.as_ptr();
        buf.clear();
        buf.prepare(32);
        buf.set_concat(&[&[0; 64]]);
        assert_eq!(buf.data.as_ptr(), before);
    }

    fn receive(
        buf: &mut ByteBuffer,
        status: &mut pictorus_peripheral_status_t,
        capacity: usize,
        result: pictorus_status_t,
        bytes: &[u8],
        reported_len: usize,
    ) -> Received {
        buf.receive(status, capacity, |ptr, cap, len| {
            let n = bytes.len().min(cap);
            unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, n) };
            unsafe { *len = reported_len };
            result
        })
    }

    #[test]
    fn receive_keeps_only_what_was_written() {
        let mut buf = ByteBuffer::new();
        let mut status = Default::default();
        assert_eq!(
            receive(&mut buf, &mut status, 8, PICTORUS_OK, &[1, 2, 3], 3),
            Received::Data
        );
        assert_eq!(buf.as_slice(), [1, 2, 3]);
        assert_eq!(status, Default::default());
    }

    #[test]
    fn receive_offers_the_requested_capacity() {
        let mut buf = ByteBuffer::new();
        let mut status = Default::default();
        let mut offered = 0;
        buf.receive(&mut status, 12, |_, cap, _| {
            offered = cap;
            PICTORUS_EMPTY
        });
        assert_eq!(offered, 12);
    }

    #[test]
    fn receive_empty_is_not_an_error() {
        let mut buf = ByteBuffer::new();
        let mut status = Default::default();
        assert_eq!(
            receive(&mut buf, &mut status, 8, PICTORUS_EMPTY, &[], 0),
            Received::Empty
        );
        assert_eq!(
            receive(&mut buf, &mut status, 8, PICTORUS_OK, &[], 0),
            Received::Empty
        );
        assert!(buf.as_slice().is_empty());
        assert_eq!(status, Default::default());
    }

    #[test]
    fn receive_failure_is_recorded_and_clears() {
        let mut buf = ByteBuffer::new();
        let mut status = Default::default();
        receive(&mut buf, &mut status, 8, PICTORUS_OK, &[9], 1);
        assert_eq!(
            receive(&mut buf, &mut status, 8, PICTORUS_ERR_PERIPHERAL, &[7], 1),
            Received::Failed
        );
        assert!(buf.as_slice().is_empty());
        assert_eq!(status.last_error, PICTORUS_ERR_PERIPHERAL);
        assert_eq!(status.error_count, 1);
    }

    #[test]
    fn receive_rejects_a_length_beyond_what_was_offered() {
        let mut buf = ByteBuffer::new();
        let mut status = Default::default();
        assert_eq!(
            receive(&mut buf, &mut status, 4, PICTORUS_OK, &[1, 2, 3, 4], 5),
            Received::Failed
        );
        assert!(buf.as_slice().is_empty());
        assert_eq!(status.last_error, PICTORUS_ERR_INVALID_ARG);
    }
}
