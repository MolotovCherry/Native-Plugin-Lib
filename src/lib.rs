use std::{error::Error, ffi::OsString, os::windows::prelude::OsStringExt, str::Utf8Error};

use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::FreeLibrary,
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES},
    },
};

/// Plugin details
#[derive(Debug, Clone)]
#[repr(C)]
pub struct Plugin {
    pub name: [u8; 128],
    pub description: [u8; 512],
}

impl Plugin {
    pub fn get_name(&self) -> Result<&str, Utf8Error> {
        let end = self
            .name
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.name.len())
            .saturating_sub(1);

        let name = &self.name[..=end];

        std::str::from_utf8(name)
    }

    pub fn get_description(&self) -> Result<&str, Utf8Error> {
        let end = self
            .description
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.description.len())
            .saturating_sub(1);

        let description = &self.description[..=end];

        std::str::from_utf8(description)
    }
}

/// Define a plugin's name and description
/// name can be 128-1 characters long, whereas
/// description can be 512-1 characters long
///
/// The last byte is reserved for \0!
#[macro_export]
macro_rules! declare_plugin {
    ($name:literal, $desc:literal) => {
        static PLUGIN: $crate::Plugin = $crate::Plugin {
            name: $crate::convert_str::<128>($name),
            description: $crate::convert_str::<512>($desc),
        };

        #[no_mangle]
        pub extern "C" fn get_plugin() -> *const $crate::Plugin {
            &PLUGIN
        }
    };
}

/// C function to get the path to the DLL, encoded as UTF16
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, or it's not a plugin made with Rust BG3 template
///
/// # Safety
/// `dll` must be a null terminated utf-16 string
#[no_mangle]
unsafe extern "C-unwind" fn get_plugin_data_c(dll: *const u16) -> *const Plugin {
    let len = (0..).take_while(|&i| *dll.offset(i) != 0).count();
    let slice = std::slice::from_raw_parts(dll, len);

    let dll = OsString::from_wide(slice);
    let dll = dll.to_str();
    if let Some(dll) = dll {
        if let Ok(plugin) = get_plugin_data(dll) {
            let plugin = Box::leak(Box::new(plugin));
            plugin as *const _
        } else {
            std::ptr::null()
        }
    } else {
        std::ptr::null()
    }
}

/// Free the memory used by Plugin
///
/// # Safety
/// Must be pointer to a valid instance of Plugin
#[no_mangle]
unsafe extern "C-unwind" fn free_plugin(plugin: *mut Plugin) {
    _ = Box::from_raw(plugin);
}

/// Rust function to get plugin data from from a plugin dll
pub fn get_plugin_data<P: AsRef<str>>(dll: P) -> Result<Plugin, Box<dyn Error>> {
    let mut path: Vec<u16> = dll.as_ref().encode_utf16().collect();
    path.push(b'\0' as u16);

    let path = PCWSTR(path.as_ptr());

    // DONT_RESOLVE_DLL_REFERENCES - Don't call DllMain when loading
    let module = unsafe { LoadLibraryExW(path, None, DONT_RESOLVE_DLL_REFERENCES)? };

    // function name we have to get the plugin details
    let symbol = "get_plugin\0";

    let get_plugin =
        unsafe { GetProcAddress(module, PCSTR(symbol.as_ptr())).ok_or("Failed to get address")? };

    let plugin = unsafe { &*(get_plugin() as *const Plugin) };
    let plugin = (*plugin).clone();

    unsafe {
        FreeLibrary(module)?;
    }

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
