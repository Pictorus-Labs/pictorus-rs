//! This module defines the FFI (Foreign Function Interface) protocol for the Pictorus PX4 library.
//!
//! The idea is to provide a way for pictorus static libs to to communicate larger amounts of data than
//! what we currently pass through FFI variables. This Rust code requires C/C++ code on the calling side
//! which is using the exact same memory layout that the rust code expects.
use crate::message_impls::{Topic, UorbMessage};
use once_cell::sync::Lazy;
use pictorus_traits::{InputBlock, OutputBlock, Pass, PassBy};
use px4_msgs_sys::orb::orb_id_t;
use spin::{RwLock, RwLockReadGuard, RwLockWriteGuard};

extern crate alloc;
use alloc::{boxed::Box, vec, vec::Vec};

use crate::message_impls::{FromPassType, ToPassType};

/// C-compatible error codes for FFI boundary
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiReturnCode {
    /// Success - no error
    Success = 0,
    /// Message length mismatch
    MessageLengthMismatch = 1,
    /// Attempt to get message type that has not been advertised
    UnadvertisedMessage = 2,
    /// Attempt to get message type that has not been subscribed
    UnsubscribedMessage = 3,
    /// Invalid message index
    InvalidMessageIndex = 4,
    /// Null argument(s) passed to function
    NullArgument = 5,
}

impl FfiReturnCode {
    /// Returns true if this represents a success state
    pub fn is_success(self) -> bool {
        self == FfiReturnCode::Success
    }

    /// Returns true if this represents an error state
    pub fn is_error(self) -> bool {
        !self.is_success()
    }
}

/// Core FFI protocol manager for PX4-Rust message exchange
///
/// This struct manages the bidirectional message passing system between PX4 C++ modules
/// and Rust computation code. It maintains separate collections of input and output messages,
/// handling memory management and synchronization safely across the FFI boundary.
///
/// # Thread Safety
///
/// The protocol is designed for single-threaded operation within PX4 modules. While it uses
/// `RwLock` for static requirements, there should be no lock contention in normal operation.
///
/// # Memory Management
///
/// All message data is stored in owned heap buffers (`Box<[u8]>`) to ensure memory safety
/// and proper cleanup. The protocol handles the complexity of C-compatible memory layout
/// while providing Rust safety guarantees.
///
/// # Examples
///
/// ```rust
/// use pictorus_px4::ffi_protocol::FfiProtocol;
/// use pictorus_px4::message_impls::SensorAccel;
///
/// // Get access to the global protocol instance
/// let mut protocol = FfiProtocol::get_mut();
///
/// // Set up message subscriptions
/// protocol.subscribe_to_message(SensorAccel::default());
/// ```
pub struct FfiProtocol {
    /// Input messages that C++ writes and Rust reads
    input_messages: Vec<MessageEntry>,
    /// Output messages that Rust writes and C++ reads  
    output_messages: Vec<MessageEntry>,
}

/// A message entry storing topic data and metadata for FFI exchange
///
/// This structure represents a single uORB topic's data within the FFI protocol.
/// It combines the topic identifier, message data buffer, and update status
/// in a memory-safe way that can be accessed from both Rust and C++ code.
///
/// # Memory Layout
///
/// The message data is stored in a heap-allocated buffer (`Box<[u8]>`) that matches
/// the exact size and layout of the corresponding PX4 C struct. This ensures binary
/// compatibility while maintaining Rust's memory safety guarantees.
///
/// # Lifecycle
///
/// 1. **Creation**: Entry is created with zero-initialized data buffer
/// 2. **Updates**: C++ writes new data, setting `updated = true`  
/// 3. **Consumption**: Rust reads data and may clear the update flag
/// 4. **Cleanup**: Drop trait ensures proper memory deallocation
pub struct MessageEntry {
    /// uORB topic identifier (pointer to static metadata)
    pub message_id: orb_id_t,
    /// Owned message data buffer with exact size for topic
    pub data: Box<[u8]>,
    /// Flag indicating whether message has been updated since last read
    pub updated: bool,
}

impl MessageEntry {
    pub fn new<T: Topic>(_topic: T) -> Self {
        let data_vec = vec![0; T::size() as usize];
        Self {
            message_id: T::id(),
            data: data_vec.into_boxed_slice(),
            updated: false,
        }
    }
}

// SAFETY: MessageEntry contains orb_id_t (pointer to static data) and owned heap data
// The orb_id_t points to static orb_metadata that's valid for the entire program
// The Box<[u8]> is owned heap-allocated data with proper Drop implementation
unsafe impl Send for MessageEntry {}
unsafe impl Sync for MessageEntry {}

// NOTE: RwLock is unnecessary overhead for single-threaded PX4 module execution,
// but required for static variable Sync requirements. In a single-threaded context,
// this will never have lock contention.
static FFI_PROTOCOL: Lazy<RwLock<FfiProtocol>> = Lazy::new(|| RwLock::new(FfiProtocol::new()));

impl FfiProtocol {
    pub fn get() -> RwLockReadGuard<'static, FfiProtocol> {
        FFI_PROTOCOL.read()
    }

    pub fn get_mut() -> RwLockWriteGuard<'static, FfiProtocol> {
        FFI_PROTOCOL.write()
    }

    fn new() -> Self {
        Self {
            input_messages: Vec::new(),
            output_messages: Vec::new(),
        }
    }

    pub fn subscribe_to_message<T: Topic>(&mut self, topic: T) {
        let entry = MessageEntry::new(topic);
        self.input_messages.push(entry);
    }

    pub fn advertise_message<T: Topic>(&mut self, topic: T) {
        let entry = MessageEntry::new(topic);
        self.output_messages.push(entry);
    }

    pub fn get_message<T: Topic>(&self) -> (Option<&T::Message>, FfiReturnCode) {
        self.input_messages
            .iter()
            .find(|entry| entry.message_id == T::id())
            .map(|entry| {
                if entry.updated {
                    (
                        Some(T::Message::view_from_bytes(&entry.data)),
                        FfiReturnCode::Success,
                    )
                } else {
                    (None, FfiReturnCode::Success)
                }
            })
            .unwrap_or((None, FfiReturnCode::UnsubscribedMessage))
    }

    pub fn set_message<T: Topic>(&mut self, message: T::Message) -> FfiReturnCode {
        if let Some(entry) = self
            .output_messages
            .iter_mut()
            .find(|e| e.message_id == T::id())
        {
            let message_bytes = message.as_bytes();

            debug_assert!(
                message_bytes.len() == entry.data.len(),
                "Message size mismatch for {:?}: expected {}, got {}",
                T::id(),
                entry.data.len(),
                message_bytes.len()
            );

            if message_bytes.len() != entry.data.len() {
                return FfiReturnCode::MessageLengthMismatch;
            }

            entry.data.copy_from_slice(message_bytes);
            entry.updated = true;
            FfiReturnCode::Success
        } else {
            FfiReturnCode::UnadvertisedMessage
        }
    }

    /// Get count of input messages that can be written to by C++
    pub fn get_input_message_count(&self) -> usize {
        self.input_messages.len()
    }

    /// Get orb_id_t for input message at given index
    pub fn get_input_message_id(&self, index: usize) -> Result<orb_id_t, FfiReturnCode> {
        self.input_messages
            .get(index)
            .map(|entry| entry.message_id)
            .ok_or(FfiReturnCode::InvalidMessageIndex)
    }

    /// Write data to input message (C++ writes input data for Rust to process)
    pub fn write_input_message(&mut self, message_id: orb_id_t, data: &[u8]) -> FfiReturnCode {
        if let Some(entry) = self
            .input_messages
            .iter_mut()
            .find(|e| e.message_id == message_id)
        {
            if data.len() != entry.data.len() {
                return FfiReturnCode::MessageLengthMismatch;
            }
            entry.data.copy_from_slice(data);
            entry.updated = true;
            FfiReturnCode::Success
        } else {
            FfiReturnCode::UnsubscribedMessage
        }
    }

    /// Get count of output messages that can be read by C++
    pub fn get_output_message_count(&self) -> usize {
        self.output_messages.len()
    }

    /// Get orb_id_t for output message at given index
    pub fn get_output_message_id(&self, index: usize) -> Result<orb_id_t, FfiReturnCode> {
        self.output_messages
            .get(index)
            .map(|entry| entry.message_id)
            .ok_or(FfiReturnCode::InvalidMessageIndex)
    }

    /// Check if output message has been updated by Rust
    pub fn output_message_has_update(&self, message_id: orb_id_t) -> Result<bool, FfiReturnCode> {
        if let Some(entry) = self
            .output_messages
            .iter()
            .find(|e| e.message_id == message_id)
        {
            Ok(entry.updated)
        } else {
            Err(FfiReturnCode::UnadvertisedMessage)
        }
    }

    /// Read output message data (C++ reads output data produced by Rust)
    pub fn read_output_message(
        &mut self,
        message_id: orb_id_t,
        buffer: &mut [u8],
    ) -> Result<usize, FfiReturnCode> {
        if let Some(entry) = self
            .output_messages
            .iter_mut()
            .find(|e| e.message_id == message_id)
        {
            if buffer.len() < entry.data.len() {
                return Err(FfiReturnCode::MessageLengthMismatch);
            }
            let data_len = entry.data.len();
            buffer[..data_len].copy_from_slice(&entry.data);
            entry.updated = false; // Mark as read
            Ok(data_len)
        } else {
            Err(FfiReturnCode::UnadvertisedMessage)
        }
    }
}

/// Pictorus output block for publishing data to PX4 uORB topics
///
/// This block takes Pictorus computation results and publishes them to PX4's uORB
/// messaging system. It converts from Pictorus data types to PX4 message structs
/// and manages the FFI protocol for safe data transfer.
///
/// # Type Parameters
///
/// * `T` - The PX4 topic type this block publishes to
///
/// # Usage
///
/// ```rust
/// use pictorus_px4::ffi_protocol::FfiOutputBlock;
/// use pictorus_px4::message_impls::VehicleAttitudeSetpoint;
///
/// // Create an output block for vehicle attitude setpoints
/// let mut output_block = FfiOutputBlock::<VehicleAttitudeSetpoint>::default();
///
/// // The block will convert Pictorus computation results to PX4 messages
/// // and publish them through the FFI protocol
/// ```
///
/// # Requirements
///
/// The topic's message type must implement [`FromPassType`]
/// to enable conversion from Pictorus data types.
pub struct FfiOutputBlock<T: Topic>
where
    T::Message: FromPassType,
{
    /// Zero-sized marker for compile-time topic identification
    _marker: core::marker::PhantomData<T>,
}

impl<T: Topic> Default for FfiOutputBlock<T>
where
    T::Message: FromPassType,
{
    fn default() -> Self {
        Self {
            _marker: core::marker::PhantomData,
        }
    }
}

/// Empty parameter struct for FFI blocks
///
/// FFI input and output blocks don't require runtime parameters since their
/// behavior is determined entirely by the topic type and FFI protocol state.
/// This struct satisfies the Pictorus block parameter requirements.
pub struct FfiBlockParameters;
impl FfiBlockParameters {
    pub fn new() -> Self {
        Self {}
    }
}

impl<T: Topic> OutputBlock for FfiOutputBlock<T>
where
    T::Message: FromPassType,
{
    type Inputs = <<T as Topic>::Message as FromPassType>::PassType;
    type Parameters = FfiBlockParameters;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        let mut protocol = FfiProtocol::get_mut();
        let result = protocol.set_message::<T>(T::Message::from_pass_type(
            context.time().as_micros() as u64,
            inputs,
        ));
        debug_assert!(
            result.is_success(),
            "Failed to set message for topic: {:?}",
            result
        );
    }
}

/// Pictorus input block for reading data from PX4 uORB topics
///
/// This block reads data from PX4's uORB messaging system and provides it to
/// Pictorus computation graphs. It manages the FFI protocol for safe data transfer
/// and converts from PX4 message structs to Pictorus data types.
///
/// # Type Parameters
///
/// * `T` - The PX4 topic type this block reads from
///
/// # Usage
///
/// ```rust
/// use pictorus_px4::ffi_protocol::FfiInputBlock;
/// use pictorus_px4::message_impls::SensorAccel;
///
/// // Create an input block for accelerometer data
/// let mut input_block = FfiInputBlock::<SensorAccel>::default();
///
/// // The block will read PX4 sensor data and convert it to Pictorus format
/// // for use in computation graphs
/// ```
///
/// # Requirements
///
/// The topic's message type must implement [`ToPassType`]
/// to enable conversion to Pictorus data types.
pub struct FfiInputBlock<T: Topic>
where
    T::Message: ToPassType,
{
    /// Cached message data in Pictorus format
    data: <<T as Topic>::Message as ToPassType>::PassType,
}

impl<T: Topic> Default for FfiInputBlock<T>
where
    T::Message: ToPassType,
{
    fn default() -> Self {
        Self {
            data: <<T as Topic>::Message as ToPassType>::PassType::default(),
        }
    }
}

impl<T: Topic> InputBlock for FfiInputBlock<T>
where
    T::Message: ToPassType,
{
    type Output = <<T as Topic>::Message as ToPassType>::PassType;
    type Parameters = FfiBlockParameters;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> PassBy<'_, Self::Output> {
        let protocol = FfiProtocol::get();

        let (data_opt, result) = protocol.get_message::<T>();
        debug_assert!(
            result.is_success(),
            "Failed to get message for topic: {:?}",
            result
        );
        if let Some(data) = data_opt {
            let (_timestamp, data) = data.to_pass_type();
            self.data = data;
        }
        self.data.as_by()
    }
}
// C-compatible FFI functions

/// Get the count of input messages registered with the FFI protocol
///
/// # Arguments
/// * `count` - Output parameter to receive the message count
///
/// # Returns
/// * `Success` - Count written to output parameter
/// * `NullArgument` - If count parameter is null
///
/// # Safety
/// The caller must ensure that:
/// - `count` points to valid memory that can be written to
/// - The pointer remains valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_input_message_count(count: *mut usize) -> FfiReturnCode {
    if count.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let protocol = FfiProtocol::get();
    *count = protocol.get_input_message_count();
    FfiReturnCode::Success
}

/// Get the uORB topic ID for an input message at the given index
///
/// # Arguments
/// * `index` - Index of the input message (0-based)
/// * `message_id` - Output parameter to receive the topic ID
///
/// # Returns
/// * `Success` - Topic ID written to output parameter
/// * `NullArgument` - If message_id parameter is null
/// * `InvalidMessageIndex` - If index is out of bounds
///
/// # Safety
/// The caller must ensure that:
/// - `message_id` points to valid memory that can be written to
/// - The pointer remains valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_input_message_id(
    index: usize,
    message_id: *mut orb_id_t,
) -> FfiReturnCode {
    if message_id.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let protocol = FfiProtocol::get();
    match protocol.get_input_message_id(index) {
        Ok(id) => {
            *message_id = id;
            FfiReturnCode::Success
        }
        Err(error) => error,
    }
}

/// Write message data to an input message buffer
///
/// # Arguments
/// * `message_id` - uORB topic ID to write to
/// * `data` - Pointer to message data buffer
/// * `len` - Length of message data in bytes
///
/// # Returns
/// * `Success` - Message data written successfully
/// * `NullArgument` - If data parameter is null
/// * `UnsubscribedMessage` - If message_id is not subscribed
/// * `MessageLengthMismatch` - If len doesn't match expected message size
///
/// # Safety
/// The caller must ensure that:
/// - `data` points to valid message data of `len` bytes
/// - The data buffer remains valid for the duration of this call
/// - The data represents a valid instance of the message type
#[no_mangle]
pub unsafe extern "C" fn rust_write_input_message(
    message_id: orb_id_t,
    data: *const u8,
    len: usize,
) -> FfiReturnCode {
    if data.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let data_slice = core::slice::from_raw_parts(data, len);
    let mut protocol = FfiProtocol::get_mut();
    protocol.write_input_message(message_id, data_slice)
}

/// Get the count of output messages registered with the FFI protocol
///
/// # Arguments
/// * `count` - Output parameter to receive the message count
///
/// # Returns
/// * `Success` - Count written to output parameter
/// * `NullArgument` - If count parameter is null
///
/// # Safety
/// The caller must ensure that:
/// - `count` points to valid memory that can be written to
/// - The pointer remains valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_output_message_count(count: *mut usize) -> FfiReturnCode {
    if count.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let protocol = FfiProtocol::get();
    *count = protocol.get_output_message_count();
    FfiReturnCode::Success
}

/// Get the uORB topic ID for an output message at the given index
///
/// # Arguments
/// * `index` - Index of the output message (0-based)
/// * `message_id` - Output parameter to receive the topic ID
///
/// # Returns
/// * `Success` - Topic ID written to output parameter
/// * `NullArgument` - If message_id parameter is null
/// * `InvalidMessageIndex` - If index is out of bounds
///
/// # Safety
/// The caller must ensure that:
/// - `message_id` points to valid memory that can be written to
/// - The pointer remains valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_get_output_message_id(
    index: usize,
    message_id: *mut orb_id_t,
) -> FfiReturnCode {
    if message_id.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let protocol = FfiProtocol::get();
    match protocol.get_output_message_id(index) {
        Ok(id) => {
            *message_id = id;
            FfiReturnCode::Success
        }
        Err(error) => error,
    }
}

/// Check if an output message has been updated by Rust
///
/// # Arguments
/// * `message_id` - uORB topic ID to check
/// * `has_update` - Output parameter to receive update status
///
/// # Returns
/// * `Success` - Update status written to output parameter
/// * `NullArgument` - If has_update parameter is null
/// * `UnadvertisedMessage` - If message_id is not advertised
///
/// # Safety
/// The caller must ensure that:
/// - `has_update` points to valid memory that can be written to
/// - The pointer remains valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_output_message_has_update(
    message_id: orb_id_t,
    has_update: *mut bool,
) -> FfiReturnCode {
    if has_update.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let protocol = FfiProtocol::get();
    match protocol.output_message_has_update(message_id) {
        Ok(updated) => {
            *has_update = updated;
            FfiReturnCode::Success
        }
        Err(error) => error,
    }
}

/// Read message data from an output message buffer
///
/// # Arguments
/// * `message_id` - uORB topic ID to read from
/// * `buffer` - Buffer to write message data to
/// * `buffer_size` - Size of the output buffer in bytes
/// * `bytes_written` - Output parameter to receive actual bytes written
///
/// # Returns
/// * `Success` - Message data read successfully, bytes_written contains actual size
/// * `NullArgument` - If buffer or bytes_written parameters are null
/// * `UnadvertisedMessage` - If message_id is not advertised
/// * `MessageLengthMismatch` - If buffer_size is too small for the message
///
/// # Safety
/// The caller must ensure that:
/// - `buffer` points to valid writable memory of at least `buffer_size` bytes
/// - `bytes_written` points to valid memory that can be written to
/// - Both pointers remain valid for the duration of this call
#[no_mangle]
pub unsafe extern "C" fn rust_read_output_message(
    message_id: orb_id_t,
    buffer: *mut u8,
    buffer_size: usize,
    bytes_written: *mut usize,
) -> FfiReturnCode {
    if buffer.is_null() || bytes_written.is_null() {
        return FfiReturnCode::NullArgument;
    }

    let buffer_slice = core::slice::from_raw_parts_mut(buffer, buffer_size);
    let mut protocol = FfiProtocol::get_mut();

    match protocol.read_output_message(message_id, buffer_slice) {
        Ok(len) => {
            *bytes_written = len;
            FfiReturnCode::Success
        }
        Err(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock message type for testing
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MockMessage {
        timestamp: u64,
        x: f32,
        y: f32,
        z: f32,
    }

    impl UorbMessage for MockMessage {
        fn view_from_bytes(bytes: &[u8]) -> &Self {
            unsafe { &*(bytes.as_ptr() as *const Self) }
        }

        fn as_bytes(&self) -> &[u8] {
            unsafe {
                core::slice::from_raw_parts(
                    self as *const Self as *const u8,
                    core::mem::size_of::<Self>(),
                )
            }
        }
    }

    // Mock topic type for testing
    #[derive(Clone, Copy, Default)]
    struct MockTopic;

    // Use Box to allocate metadata on heap and leak it for static lifetime
    static mut MOCK_METADATA_PTR: Option<*const px4_msgs_sys::orb::orb_metadata> = None;
    static INIT_ONCE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

    impl Topic for MockTopic {
        type Message = MockMessage;

        fn id() -> px4_msgs_sys::orb::orb_id_t {
            unsafe {
                if !INIT_ONCE.load(core::sync::atomic::Ordering::Acquire) {
                    let metadata = Box::new(px4_msgs_sys::orb::orb_metadata {
                        o_name: b"mock_topic\0".as_ptr() as *const core::ffi::c_char,
                        o_size: core::mem::size_of::<MockMessage>() as u16,
                        o_size_no_padding: core::mem::size_of::<MockMessage>() as u16,
                        message_hash: 0x12345678,
                        o_id: 1,
                        o_queue: 1,
                    });
                    MOCK_METADATA_PTR = Some(Box::into_raw(metadata));
                    INIT_ONCE.store(true, core::sync::atomic::Ordering::Release);
                }
                MOCK_METADATA_PTR.unwrap()
            }
        }
    }

    fn create_test_message() -> MockMessage {
        MockMessage {
            timestamp: 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[test]
    fn test_ffi_return_code_success() {
        assert!(FfiReturnCode::Success.is_success());
        assert!(!FfiReturnCode::Success.is_error());
    }

    #[test]
    fn test_ffi_return_code_errors() {
        let error_codes = [
            FfiReturnCode::MessageLengthMismatch,
            FfiReturnCode::UnadvertisedMessage,
            FfiReturnCode::UnsubscribedMessage,
            FfiReturnCode::InvalidMessageIndex,
            FfiReturnCode::NullArgument,
        ];

        for code in error_codes {
            assert!(!code.is_success());
            assert!(code.is_error());
        }
    }

    #[test]
    fn test_message_entry_new() {
        let entry = MessageEntry::new(MockTopic::default());
        assert_eq!(entry.message_id, MockTopic::id());
        assert_eq!(entry.data.len(), MockTopic::size() as usize);
        assert!(!entry.updated);
    }

    #[test]
    fn test_ffi_protocol_new() {
        let protocol = FfiProtocol::new();
        assert_eq!(protocol.get_input_message_count(), 0);
        assert_eq!(protocol.get_output_message_count(), 0);
    }

    #[test]
    fn test_subscribe_to_message() {
        let mut protocol = FfiProtocol::new();
        protocol.subscribe_to_message(MockTopic::default());

        assert_eq!(protocol.get_input_message_count(), 1);
        assert_eq!(protocol.get_input_message_id(0), Ok(MockTopic::id()));
    }

    #[test]
    fn test_advertise_message() {
        let mut protocol = FfiProtocol::new();
        protocol.advertise_message(MockTopic::default());

        assert_eq!(protocol.get_output_message_count(), 1);
        assert_eq!(protocol.get_output_message_id(0), Ok(MockTopic::id()));
    }

    #[test]
    fn test_get_message_unsubscribed() {
        let protocol = FfiProtocol::new();
        let (message, result) = protocol.get_message::<MockTopic>();

        assert!(message.is_none());
        assert_eq!(result, FfiReturnCode::UnsubscribedMessage);
    }

    #[test]
    fn test_get_message_no_update() {
        let mut protocol = FfiProtocol::new();
        protocol.subscribe_to_message(MockTopic::default());

        let (message, result) = protocol.get_message::<MockTopic>();
        assert!(message.is_none());
        assert_eq!(result, FfiReturnCode::Success);
    }

    #[test]
    fn test_set_message_unadvertised() {
        let mut protocol = FfiProtocol::new();
        let message = create_test_message();
        let result = protocol.set_message::<MockTopic>(message);

        assert_eq!(result, FfiReturnCode::UnadvertisedMessage);
    }

    #[test]
    fn test_set_and_get_message() {
        let mut protocol = FfiProtocol::new();
        protocol.advertise_message(MockTopic::default());
        protocol.subscribe_to_message(MockTopic::default());

        let mut test_message = create_test_message();
        test_message.timestamp = 12345;
        test_message.x = 1.0;
        test_message.y = 2.0;
        test_message.z = 3.0;

        let result = protocol.set_message::<MockTopic>(test_message);
        assert_eq!(result, FfiReturnCode::Success);

        // Simulate C++ writing the same data to input
        let message_bytes = test_message.as_bytes();
        let write_result = protocol.write_input_message(MockTopic::id(), message_bytes);
        assert_eq!(write_result, FfiReturnCode::Success);

        let (retrieved_message, get_result) = protocol.get_message::<MockTopic>();
        assert_eq!(get_result, FfiReturnCode::Success);
        assert!(retrieved_message.is_some());

        let retrieved = retrieved_message.unwrap();
        assert_eq!(retrieved.timestamp, 12345);
        assert_eq!(retrieved.x, 1.0);
        assert_eq!(retrieved.y, 2.0);
        assert_eq!(retrieved.z, 3.0);
    }

    #[test]
    fn test_write_input_message_length_mismatch() {
        let mut protocol = FfiProtocol::new();
        protocol.subscribe_to_message(MockTopic::default());

        let wrong_size_data = vec![0u8; 10]; // Wrong size
        let result = protocol.write_input_message(MockTopic::id(), &wrong_size_data);
        assert_eq!(result, FfiReturnCode::MessageLengthMismatch);
    }

    #[test]
    fn test_write_input_message_unsubscribed() {
        let mut protocol = FfiProtocol::new();
        let data = vec![0u8; MockTopic::size() as usize];
        let result = protocol.write_input_message(MockTopic::id(), &data);
        assert_eq!(result, FfiReturnCode::UnsubscribedMessage);
    }

    #[test]
    fn test_get_input_message_id_invalid_index() {
        let protocol = FfiProtocol::new();
        let result = protocol.get_input_message_id(999);
        assert_eq!(result, Err(FfiReturnCode::InvalidMessageIndex));
    }

    #[test]
    fn test_get_output_message_id_invalid_index() {
        let protocol = FfiProtocol::new();
        let result = protocol.get_output_message_id(999);
        assert_eq!(result, Err(FfiReturnCode::InvalidMessageIndex));
    }

    #[test]
    fn test_output_message_has_update() {
        let mut protocol = FfiProtocol::new();
        protocol.advertise_message(MockTopic::default());

        // Initially no update
        let result = protocol.output_message_has_update(MockTopic::id());
        assert_eq!(result, Ok(false));

        // After setting message, should have update
        let test_message = create_test_message();
        protocol.set_message::<MockTopic>(test_message);

        let result = protocol.output_message_has_update(MockTopic::id());
        assert_eq!(result, Ok(true));
    }

    #[test]
    fn test_output_message_has_update_unadvertised() {
        let protocol = FfiProtocol::new();
        let result = protocol.output_message_has_update(MockTopic::id());
        assert_eq!(result, Err(FfiReturnCode::UnadvertisedMessage));
    }

    #[test]
    fn test_read_output_message() {
        let mut protocol = FfiProtocol::new();
        protocol.advertise_message(MockTopic::default());

        let mut test_message = create_test_message();
        test_message.timestamp = 54321;
        test_message.x = 4.0;

        protocol.set_message::<MockTopic>(test_message);

        let mut buffer = vec![0u8; MockTopic::size() as usize];
        let result = protocol.read_output_message(MockTopic::id(), &mut buffer);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MockTopic::size() as usize);

        // Verify the data was copied correctly
        let read_message = MockMessage::view_from_bytes(&buffer);
        assert_eq!(read_message.timestamp, 54321);
        assert_eq!(read_message.x, 4.0);

        // After reading, update flag should be cleared
        let has_update = protocol.output_message_has_update(MockTopic::id());
        assert_eq!(has_update, Ok(false));
    }

    #[test]
    fn test_read_output_message_buffer_too_small() {
        let mut protocol = FfiProtocol::new();
        protocol.advertise_message(MockTopic::default());

        let test_message = create_test_message();
        protocol.set_message::<MockTopic>(test_message);

        let mut small_buffer = vec![0u8; 5]; // Too small
        let result = protocol.read_output_message(MockTopic::id(), &mut small_buffer);

        assert_eq!(result, Err(FfiReturnCode::MessageLengthMismatch));
    }

    #[test]
    fn test_read_output_message_unadvertised() {
        let mut protocol = FfiProtocol::new();
        let mut buffer = vec![0u8; MockTopic::size() as usize];
        let result = protocol.read_output_message(MockTopic::id(), &mut buffer);

        assert_eq!(result, Err(FfiReturnCode::UnadvertisedMessage));
    }

    #[test]
    fn test_multiple_messages() {
        let mut protocol = FfiProtocol::new();
        protocol.subscribe_to_message(MockTopic::default());
        protocol.advertise_message(MockTopic::default());

        // Test multiple message operations
        assert_eq!(protocol.get_input_message_count(), 1);
        assert_eq!(protocol.get_output_message_count(), 1);

        // Set and retrieve multiple times
        for i in 1..=5 {
            let mut test_message = create_test_message();
            test_message.timestamp = i as u64;

            let result = protocol.set_message::<MockTopic>(test_message);
            assert_eq!(result, FfiReturnCode::Success);

            let has_update = protocol.output_message_has_update(MockTopic::id());
            assert_eq!(has_update, Ok(true));
        }
    }

    #[test]
    fn test_concurrent_access() {
        // Test that we can get read and write guards
        {
            let _read_guard = FfiProtocol::get();
            // Multiple read guards should be possible
            let _read_guard2 = FfiProtocol::get();
        }

        {
            let mut write_guard = FfiProtocol::get_mut();
            write_guard.subscribe_to_message(MockTopic::default());
        }

        // Verify the subscription was added
        let read_guard = FfiProtocol::get();
        assert_eq!(read_guard.get_input_message_count(), 1);
    }

    // FFI function tests using unsafe code
    #[test]
    fn test_ffi_get_input_message_count() {
        // Reset protocol state
        {
            let mut protocol = FfiProtocol::get_mut();
            protocol.input_messages.clear();
            protocol.subscribe_to_message(MockTopic::default());
        }

        let mut count = 0usize;
        let result = unsafe { rust_get_input_message_count(&mut count) };

        assert_eq!(result, FfiReturnCode::Success);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_ffi_get_input_message_count_null() {
        let result = unsafe { rust_get_input_message_count(core::ptr::null_mut()) };
        assert_eq!(result, FfiReturnCode::NullArgument);
    }

    #[test]
    fn test_ffi_get_input_message_id() {
        {
            let mut protocol = FfiProtocol::get_mut();
            protocol.input_messages.clear();
            protocol.subscribe_to_message(MockTopic::default());
        }

        let mut message_id: orb_id_t = core::ptr::null();
        let result = unsafe { rust_get_input_message_id(0, &mut message_id) };

        assert_eq!(result, FfiReturnCode::Success);
        assert_eq!(message_id, MockTopic::id());
    }

    #[test]
    fn test_ffi_get_input_message_id_invalid_index() {
        let mut message_id: orb_id_t = core::ptr::null();
        let result = unsafe { rust_get_input_message_id(999, &mut message_id) };

        assert_eq!(result, FfiReturnCode::InvalidMessageIndex);
    }

    #[test]
    fn test_ffi_write_input_message() {
        {
            let mut protocol = FfiProtocol::get_mut();
            protocol.input_messages.clear();
            protocol.subscribe_to_message(MockTopic::default());
        }

        let test_data = vec![42u8; MockTopic::size() as usize];
        let result = unsafe {
            rust_write_input_message(MockTopic::id(), test_data.as_ptr(), test_data.len())
        };

        assert_eq!(result, FfiReturnCode::Success);

        // Verify data was written
        let protocol = FfiProtocol::get();
        let entry = protocol
            .input_messages
            .iter()
            .find(|e| e.message_id == MockTopic::id())
            .unwrap();
        assert!(entry.updated);
        assert_eq!(entry.data[0], 42);
    }

    #[test]
    fn test_ffi_write_input_message_null() {
        let result = unsafe { rust_write_input_message(MockTopic::id(), core::ptr::null(), 0) };

        assert_eq!(result, FfiReturnCode::NullArgument);
    }

    #[test]
    fn test_ffi_output_message_has_update() {
        {
            let mut protocol = FfiProtocol::get_mut();
            protocol.output_messages.clear();
            protocol.advertise_message(MockTopic::default());
        }

        let mut has_update = false;
        let result = unsafe { rust_output_message_has_update(MockTopic::id(), &mut has_update) };

        assert_eq!(result, FfiReturnCode::Success);
        assert!(!has_update);
    }

    #[test]
    fn test_ffi_read_output_message() {
        {
            let mut protocol = FfiProtocol::get_mut();
            protocol.output_messages.clear();
            protocol.advertise_message(MockTopic::default());

            let test_message = create_test_message();
            protocol.set_message::<MockTopic>(test_message);
        }

        let mut buffer = vec![0u8; MockTopic::size() as usize];
        let mut bytes_written = 0usize;

        let result = unsafe {
            rust_read_output_message(
                MockTopic::id(),
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut bytes_written,
            )
        };

        assert_eq!(result, FfiReturnCode::Success);
        assert_eq!(bytes_written, MockTopic::size() as usize);
    }

    #[test]
    fn test_ffi_read_output_message_null_args() {
        let result = unsafe {
            rust_read_output_message(
                MockTopic::id(),
                core::ptr::null_mut(),
                0,
                core::ptr::null_mut(),
            )
        };

        assert_eq!(result, FfiReturnCode::NullArgument);
    }
}
