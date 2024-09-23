use std::{
    ffi::OsString,
    os::windows::prelude::OsStringExt as _,
    ptr::{self, addr_of},
    slice,
};

use crate::{get_plugin_data as _get_plugin_data, Plugin, PluginGuard as RPluginGuard};

/// The real plugin guard
#[repr(C)]
struct _PluginGuard<'a> {
    data: *const Plugin<'a>,
    _guard: RPluginGuard<'a>,
}

/// Guard for the plugin data
/// `data` will be invalid when guard is freed
#[repr(C)]
struct PluginGuard<'a> {
    data: *const Plugin<'a>,
}

/// Get a plugin's data
///
/// Takes in a path to the dll, encoded as UTF16, with length `len`
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, it's not a plugin made with Rust Native template,
/// or the file does not exist.
///
/// # Safety
/// `len` must be the correct. this is the number of u16 elems, _not_ the number of bytes
#[no_mangle]
unsafe extern "C" fn get_plugin_data<'a>(dll: *const u16, len: usize) -> *const PluginGuard<'a> {
    let slice = unsafe { slice::from_raw_parts(dll, len) };

    let dll = OsString::from_wide(slice);
    let dll = dll.to_str().map(_get_plugin_data);

    match dll {
        Some(Ok(plugin)) => {
            let ptr = addr_of!(plugin.module.1);
            let plugin = _PluginGuard {
                data: ptr,
                _guard: plugin,
            };

            let plugin = Box::leak(Box::new(plugin));

            // it leads to UB to directly cast to *const here, or to reborrow as shared ref,
            // as it loses its rw provenance
            plugin as *mut _ as _
        }

        _ => ptr::null(),
    }
}

/// Free the memory used by PluginGuard
///
/// # Safety
/// Must be pointer to a valid instance of PluginGuard
#[no_mangle]
unsafe extern "C" fn free_plugin(plugin: *const PluginGuard) {
    drop(unsafe { Box::from_raw(plugin as *mut _PluginGuard) });
}
