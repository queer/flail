#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

unsafe impl Send for struct_io_manager {}
unsafe impl Sync for struct_io_manager {}

// TODO: Validate my assumptions around thread-safety...
unsafe impl Send for struct_ext2_filsys {}
unsafe impl Sync for struct_ext2_filsys {}
