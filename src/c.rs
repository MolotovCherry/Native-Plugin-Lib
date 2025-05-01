use std::{
    ffi::{OsString, c_char},
    os::windows::prelude::OsStringExt as _,
    slice,
};

use crate::{PluginData, Version};

/// Get a plugin's data
///
/// Takes in a path to the dll, encoded as UTF16, with length `len`
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
/// the file does not exist, or you need to update the native plugin lib since the data version is too high
///
/// # Safety
/// `len` must be the correct. this is the number of u16 elems, _not_ the number of bytes
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_data(dll: *const u16, len: usize) -> Option<Box<PluginData>> {
    let slice = unsafe { slice::from_raw_parts(dll, len) };

    let dll = OsString::from_wide(slice);
    dll.to_str()
        .map(PluginData::new)
        .transpose()
        .ok()
        .flatten()
        .map(Box::new)
}

/// Get the plugin name
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_name(data: *mut PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().name.ptr.as_ptr()
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_author(data: *mut PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().author.ptr.as_ptr()
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_description(data: *mut PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().description.ptr.as_ptr()
}

/// Get the plugin version
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_version(data: *mut PluginData) -> *const Version {
    let data = unsafe { &*data };
    &data.plugin().version
}

/// Free the memory used by PluginData. This is only valid for pointers made by get_plugin_data
#[unsafe(no_mangle)]
extern "C" fn free_plugin_data(_: Option<Box<PluginData>>) {}
