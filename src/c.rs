use std::{ffi::OsString, os::windows::prelude::OsStringExt as _, ptr, slice};

use crate::{get_plugin_data as _get_plugin_data, PluginGuard, RStr, Version};

/// Get a plugin's data
///
/// Takes in a path to the dll, encoded as UTF16, with length `len`
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
/// or the file does not exist.
///
/// # Safety
/// `len` must be the correct size
#[no_mangle]
unsafe extern "C" fn get_plugin_data(dll: *const u16, len: usize) -> *const PluginGuard<'static> {
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
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn name(plugin: *const PluginGuard<'static>) -> RStr {
    let plugin = unsafe { &*plugin };
    plugin.data().name
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn author(plugin: *const PluginGuard<'static>) -> RStr {
    let plugin = unsafe { &*plugin };
    plugin.data().author
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn description(plugin: *const PluginGuard<'static>) -> RStr {
    let plugin = unsafe { &*plugin };
    plugin.data().description
}

/// Get the plugin version
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn version(plugin: *const PluginGuard<'static>) -> *const Version {
    let plugin = unsafe { &*plugin };
    &plugin.data().version
}

/// Free the memory used by PluginGuard
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn free_plugin(plugin: *const PluginGuard<'_>) {
    drop(unsafe { Box::from_raw(plugin as *mut PluginGuard) });
}
