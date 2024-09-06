use std::{
    ffi::{c_char, CString, NulError, OsString},
    os::windows::prelude::OsStringExt as _,
    ptr, slice,
};

use crate::{get_plugin_data as _get_plugin_data, Plugin, Version};

/// Plugin details
#[derive(Debug)]
pub struct CPlugin {
    pub name: CString,
    pub author: CString,
    pub description: CString,
    pub version: Version,
}

impl<'a> TryFrom<Plugin<'a>> for CPlugin {
    type Error = NulError;

    fn try_from(value: Plugin) -> Result<Self, Self::Error> {
        let s = Self {
            name: CString::new(value.name.as_str())?,
            author: CString::new(value.author.as_str())?,
            description: CString::new(value.description.as_str())?,
            version: value.version,
        };

        Ok(s)
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
unsafe extern "C" fn get_plugin_data(dll: *const u16) -> *const CPlugin {
    let len = (0..)
        .take_while(|&i| unsafe { *dll.offset(i) } != 0)
        .count();
    let slice = unsafe { slice::from_raw_parts(dll, len) };

    let dll = OsString::from_wide(slice);
    let dll = dll.to_str().map(_get_plugin_data);

    match dll {
        Some(Ok(plugin)) => {
            let Ok(plugin): Result<CPlugin, _> = plugin.plugin.try_into() else {
                return ptr::null();
            };

            let plugin = Box::leak(Box::new(plugin));
            // it leads to UB to directly cast to *const here, or to reborrow as shared ref,
            // as it loses its rw provenance
            plugin as *mut _ as *const _
        }

        _ => ptr::null(),
    }
}

/// Get the plugin name
///
/// # Safety
/// Must be pointer to a valid instance of CPlugin
#[no_mangle]
unsafe extern "C" fn name(plugin: *const CPlugin) -> *const c_char {
    let plugin = unsafe { &*plugin };
    plugin.name.as_ptr()
}

/// Get the plugin author
///
/// # Safety
/// Must be pointer to a valid instance of CPlugin
#[no_mangle]
unsafe extern "C" fn author(plugin: *const CPlugin) -> *const c_char {
    let plugin = unsafe { &*plugin };
    plugin.author.as_ptr()
}

/// Get the plugin description
///
/// # Safety
/// Must be pointer to a valid instance of CPlugin
#[no_mangle]
unsafe extern "C" fn description(plugin: *const CPlugin) -> *const c_char {
    let plugin = unsafe { &*plugin };
    plugin.description.as_ptr()
}

/// Get the plugin version
///
/// # Safety
/// Must be pointer to a valid instance of CPlugin
#[no_mangle]
unsafe extern "C" fn version(plugin: *const CPlugin) -> *const Version {
    let plugin = unsafe { &*plugin };
    &plugin.version
}

/// Free the memory used by CPlugin
///
/// # Safety
/// Must be pointer to a valid instance of CPlugin
#[no_mangle]
unsafe extern "C" fn free_plugin(plugin: *const CPlugin) {
    drop(unsafe { Box::from_raw(plugin as *mut CPlugin) });
}
