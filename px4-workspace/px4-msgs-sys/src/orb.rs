pub type orb_id_size_t = u16;
#[doc = " Object metadata."]
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct orb_metadata {
    #[doc = "< unique object name"]
    pub o_name: *const ::core::ffi::c_char,
    #[doc = "< object size"]
    pub o_size: u16,
    #[doc = "< object size w/o padding at the end (for logger)"]
    pub o_size_no_padding: u16,
    #[doc = "< Hash over all fields for message compatibility checks"]
    pub message_hash: u32,
    #[doc = "< ORB_ID enum"]
    pub o_id: orb_id_size_t,
    #[doc = "< queue size"]
    pub o_queue: u8,
}

impl PartialEq for orb_metadata {
    fn eq(&self, other: &Self) -> bool {
        self.o_id == other.o_id
    }
}

// #[allow(clippy::unnecessary_operation, clippy::identity_op)]
// const _: () = {
//     ["Size of orb_metadata"][::core::mem::size_of::<orb_metadata>() - 16usize];
//     ["Alignment of orb_metadata"][::core::mem::align_of::<orb_metadata>() - 4usize];
//     ["Offset of field: orb_metadata::o_name"]
//         [::core::mem::offset_of!(orb_metadata, o_name) - 0usize];
//     ["Offset of field: orb_metadata::o_size"]
//         [::core::mem::offset_of!(orb_metadata, o_size) - 4usize];
//     ["Offset of field: orb_metadata::o_size_no_padding"]
//         [::core::mem::offset_of!(orb_metadata, o_size_no_padding) - 6usize];
//     ["Offset of field: orb_metadata::message_hash"]
//         [::core::mem::offset_of!(orb_metadata, message_hash) - 8usize];
//     ["Offset of field: orb_metadata::o_id"][::core::mem::offset_of!(orb_metadata, o_id) - 12usize];
//     ["Offset of field: orb_metadata::o_queue"]
//         [::core::mem::offset_of!(orb_metadata, o_queue) - 14usize];
// };
pub type orb_id_t = *const orb_metadata;
unsafe extern "C" {
    pub fn uorb_start() -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn uorb_status() -> ::core::ffi::c_int;
}
unsafe extern "C" {
    pub fn uorb_top(
        topic_filter: *mut *mut ::core::ffi::c_char,
        num_filters: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
#[doc = " ORB topic advertiser handle.\n\n Advertiser handles are global; once obtained they can be shared freely\n and do not need to be closed or released.\n\n This permits publication from interrupt context and other contexts where\n a file-descriptor-based handle would not otherwise be in scope for the\n publisher."]
pub type orb_advert_t = *mut ::core::ffi::c_void;
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_advertise()"]
    pub fn orb_advertise(
        meta: *const orb_metadata,
        data: *const ::core::ffi::c_void,
    ) -> orb_advert_t;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_advertise_multi()"]
    pub fn orb_advertise_multi(
        meta: *const orb_metadata,
        data: *const ::core::ffi::c_void,
        instance: *mut ::core::ffi::c_int,
    ) -> orb_advert_t;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_unadvertise()"]
    pub fn orb_unadvertise(handle: orb_advert_t) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_publish()"]
    pub fn orb_publish(
        meta: *const orb_metadata,
        handle: orb_advert_t,
        data: *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_subscribe()"]
    pub fn orb_subscribe(meta: *const orb_metadata) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_subscribe_multi()"]
    pub fn orb_subscribe_multi(
        meta: *const orb_metadata,
        instance: ::core::ffi::c_uint,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_unsubscribe()"]
    pub fn orb_unsubscribe(handle: ::core::ffi::c_int) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_copy()"]
    pub fn orb_copy(
        meta: *const orb_metadata,
        handle: ::core::ffi::c_int,
        buffer: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_check()"]
    pub fn orb_check(handle: ::core::ffi::c_int, updated: *mut bool) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_exists()"]
    pub fn orb_exists(
        meta: *const orb_metadata,
        instance: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " Get the number of published instances of a topic group\n\n @param meta    ORB topic metadata.\n @return    The number of published instances of this topic"]
    pub fn orb_group_count(meta: *const orb_metadata) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_set_interval()"]
    pub fn orb_set_interval(
        handle: ::core::ffi::c_int,
        interval: ::core::ffi::c_uint,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " @see uORB::Manager::orb_get_interval()"]
    pub fn orb_get_interval(
        handle: ::core::ffi::c_int,
        interval: *mut ::core::ffi::c_uint,
    ) -> ::core::ffi::c_int;
}
unsafe extern "C" {
    #[doc = " Returns the C type string from a short type in message fields metadata, or nullptr\n if not a short type"]
    pub fn orb_get_c_type(short_type: ::core::ffi::c_uchar) -> *const ::core::ffi::c_char;
}
unsafe extern "C" {
    #[doc = " Returns the queue size of a topic\n @param meta orb topic metadata"]
    pub fn orb_get_queue_size(meta: *const orb_metadata) -> u8;
}
unsafe extern "C" {
    #[doc = " Print a topic to console. Do not call this directly, use print_message() instead.\n @param meta orb topic metadata\n @param data expected to be aligned to the largest member"]
    pub fn orb_print_message_internal(
        meta: *const orb_metadata,
        data: *const ::core::ffi::c_void,
        print_topic_name: bool,
    );
}
pub type arming_state_t = u8;
pub type main_state_t = u8;
pub type hil_state_t = u8;
pub type navigation_state_t = u8;
pub type switch_pos_t = u8;
