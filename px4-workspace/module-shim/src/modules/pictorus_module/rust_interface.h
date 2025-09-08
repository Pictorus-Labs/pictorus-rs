#pragma once

#include <uORB/uORB.h>


/// C-compatible error codes for FFI boundary (matches Rust FfiReturnCode)
typedef enum {
    /// Success - no error
    FFI_SUCCESS = 0,
    /// Message length mismatch
    FFI_MESSAGE_LENGTH_MISMATCH = 1,
    /// Attempt to get message type that has not been advertised
    FFI_UNADVERTISED_MESSAGE = 2,
    /// Attempt to get message type that has not been subscribed
    FFI_UNSUBSCRIBED_MESSAGE = 3,
    /// Invalid message index
    FFI_INVALID_MESSAGE_INDEX = 4,
    /// Null argument(s) passed to function
    FFI_NULL_ARGUMENT = 5,
} FfiError;

extern "C" {
// Message Input (i.e. C++ inputting to Rust)
FfiError rust_get_input_message_count(size_t* count);
FfiError rust_get_input_message_id(size_t index, orb_id_t* message_id);
FfiError rust_write_input_message(orb_id_t message_id, const uint8_t* data, size_t len);
// Message Output (i.e. C++ taking output from Rust)
FfiError rust_get_output_message_count(size_t* count);  
FfiError rust_get_output_message_id(size_t index, orb_id_t* message_id);
FfiError rust_output_message_has_update(orb_id_t message_id, bool* has_update);
FfiError rust_read_output_message(orb_id_t message_id, uint8_t* buffer, size_t buffer_size, size_t* bytes_written);
}