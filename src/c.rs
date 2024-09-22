use std::{
    ffi::{c_char, OsString},
    os::windows::prelude::OsStringExt as _,
    ptr, slice,
};

use abi_stable::std_types::RStr;

use crate::{get_plugin_data as _get_plugin_data, PluginGuard, Version};

/// A plugin string.
///
/// # Safety
/// This points to a valid utf-8 string
/// This does not contain a null terminator
/// This is only valid for reads up to `len`
#[repr(C)]
struct PluginStr {
    ptr: *const c_char,
    len: usize,
}

impl PluginStr {
    fn new(data: &RStr<'_>) -> Self {
        Self {
            ptr: data.as_ptr().cast(),
            len: data.len(),
        }
    }
}

/// Get a plugin's data
///
/// Takes in a path to the dll, encoded as UTF16
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
/// or the file does not exist.
///
/// # Safety
/// `dll` must be a null terminated utf-16 string
#[no_mangle]
unsafe extern "C" fn get_plugin_data(dll: *const u16) -> *const PluginGuard<'static> {
    let len = (0..)
        .take_while(|&i| unsafe { *dll.offset(i) } != 0)
        .count();
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
unsafe extern "C" fn name(plugin: *const PluginGuard<'static>) -> PluginStr {
    let plugin = unsafe { &*plugin };
    PluginStr::new(&plugin.data().name)
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn author(plugin: *const PluginGuard<'static>) -> PluginStr {
    let plugin = unsafe { &*plugin };
    PluginStr::new(&plugin.data().author)
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn description(plugin: *const PluginGuard<'static>) -> PluginStr {
    let plugin = unsafe { &*plugin };
    PluginStr::new(&plugin.data().description)
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
