use std::{
    ffi::{OsString, c_char},
    os::windows::prelude::OsStringExt as _,
    ptr, slice,
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
unsafe extern "C" fn get_plugin_data(dll: *const u16, len: usize) -> *const PluginData {
    let slice = unsafe { slice::from_raw_parts(dll, len) };

    let dll = OsString::from_wide(slice);
    let data = dll
        .to_str()
        .map(PluginData::new)
        .transpose()
        .ok()
        .flatten()
        .map(Box::new)
        .map(Box::into_raw);

    match data {
        Some(data) => data,
        None => ptr::null(),
    }
}

/// Get the plugin name
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
extern "C" fn get_plugin_name(data: &PluginData) -> *const c_char {
    data.plugin().name.as_ptr()
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
extern "C" fn get_plugin_author(data: &PluginData) -> *const c_char {
    data.plugin().author.as_ptr()
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
extern "C" fn get_plugin_description(data: &PluginData) -> *const c_char {
    data.plugin().description.as_ptr()
}

/// Get the plugin version
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
extern "C" fn get_plugin_version(data: &PluginData) -> &Version {
    &data.plugin_ref().version
}

/// Free the memory used by PluginData.
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
extern "C" fn free_plugin_data(data: *const PluginData) {
    if !data.is_null() {
        let data = unsafe { Box::from_raw(data.cast_mut()) };
        drop(data);
    }
}
