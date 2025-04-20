use std::{
    ffi::{OsString, c_char},
    os::windows::prelude::OsStringExt as _,
    ptr, slice,
};

use crate::{PluginData, Version, get_plugin_data as _get_plugin_data};

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
    let dll = dll.to_str().map(_get_plugin_data);

    match dll {
        Some(Ok(plugin)) => {
            let plugin = Box::leak(Box::new(plugin));

            // it leads to UB to directly cast to *const here, or to reborrow as shared ref,
            // as it loses its rw provenance
            plugin as *mut _ as _
        }

        _ => ptr::null(),
    }
}

/// Get the plugin name
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_name(data: *const PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().name.ptr.as_ptr()
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_author(data: *const PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().author.ptr.as_ptr()
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_description(data: *const PluginData) -> *const c_char {
    let data = unsafe { &*data };
    data.plugin().description.ptr.as_ptr()
}

/// Get the plugin version
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn get_plugin_version(data: *const PluginData) -> *const Version {
    let data = unsafe { &*data };
    &data.plugin().version
}

/// Free the memory used by PluginData
///
/// # Safety
/// Must be pointer to a valid instance of PluginData
#[unsafe(no_mangle)]
unsafe extern "C" fn free_plugin_data(data: *const PluginData) {
    let data = unsafe { Box::from_raw(data as *mut PluginData) };
    drop(data);
}
