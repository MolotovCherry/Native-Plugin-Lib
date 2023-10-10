use std::{
    ffi::{OsStr, OsString},
    os::windows::prelude::OsStringExt,
};

use libloading::{Library, Symbol};

/// Plugin details
#[repr(C)]
pub struct Plugin {
    name: [u8; 128],
    description: [u8; 512],
}

/// Define a plugin's name and description
/// name can be 128-1 characters long, whereas
/// description can be 512-1 characters long
///
/// The last byte is reserved for \0!
#[macro_export]
macro_rules! plugin {
    ($name:literal, $desc:literal) => {
        #[no_mangle]
        static PLUGIN: Plugin = Plugin {
            name: $crate::convert_str::<128>($name),
            description: $crate::convert_str::<512>($desc),
        };
    };
}

/// C function to get the path to the DLL, encoded as UTF16
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, or it's not a plugin made with Rust BG3 template
///
/// # Safety
/// `dll` must be a null terminated utf-16 string
#[no_mangle]
pub unsafe extern "C-unwind" fn get_plugin_data_c(dll: *const u16) -> *const Plugin {
    let len = (0..).take_while(|&i| *dll.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(dll, len);

    let dll = OsString::from_wide(slice);
    if let Ok(plugin) = get_plugin_data(dll) {
        let plugin = Box::leak(Box::new(plugin));
        plugin as *const _
    } else {
        std::ptr::null()
    }
}

/// Free the memory used by Plugin
///
/// # Safety
/// Must be pointer to a valid instance of Plugin
#[no_mangle]
pub unsafe extern "C-unwind" fn free_plugin(plugin: *mut Plugin) {
    _ = Box::from_raw(plugin);
}

/// Rust function to get plugin data from from a plugin dll
pub fn get_plugin_data<P: AsRef<OsStr>>(dll: P) -> Result<Plugin, libloading::Error> {
    let plugin = unsafe {
        let lib = Library::new(dll)?;
        let data: Symbol<Plugin> = lib.get(b"PLUGIN\0")?;

        Plugin {
            name: data.name,
            description: data.description,
        }
    };

    Ok(plugin)
}

/// Convert static string to compile time array
pub const fn convert_str<const N: usize>(string: &'static str) -> [u8; N] {
    assert!(
        string.len() <= N,
        "String len must be <= total available space"
    );

    let mut arr = [0u8; N];
    let mut i = 0;
    let bytes = string.as_bytes();

    let len = bytes.len();
    while i < len {
        arr[i] = bytes[i];
        i += 1;
    }

    arr
}
