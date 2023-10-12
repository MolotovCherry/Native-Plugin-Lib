use std::{
    error::Error, ffi::OsString, os::windows::prelude::OsStringExt, path::Path, str::Utf8Error,
};

use konst::{primitive::parse_u16, unwrap_ctx};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::FreeLibrary,
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES},
    },
};

/// Plugin details
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Plugin {
    pub name: [u8; 128],
    pub author: [u8; 50],
    pub description: [u8; 512],
    pub version: Version,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
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

    pub fn get_author(&self) -> Result<&str, Utf8Error> {
        let end = self
            .author
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.author.len())
            .saturating_sub(1);

        let author = &self.author[..=end];

        std::str::from_utf8(author)
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
/// name can be 128 characters long, whereas
/// description can be 512 characters long
#[macro_export]
macro_rules! declare_plugin {
    ($name:literal, $desc:literal) => {
        $crate::declare_plugin!($name, "", $desc);
    };

    ($name:literal, $author:literal, $desc:literal) => {
        #[no_mangle]
        static PLUGIN_DATA: $crate::Plugin = $crate::Plugin {
            name: $crate::convert_str::<128>($name),
            author: $crate::convert_str::<50>($author),
            description: $crate::convert_str::<512>($desc),
            version: $crate::Version {
                major: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MAJOR")),
                minor: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MINOR")),
                patch: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_PATCH")),
            },
        };
    };
}

/// Get a plugin's data
///
/// Takes in a path to the dll, encoded as UTF16
/// Returns null pointer if it failed, non-null if it succeeded.
/// If it failed, either the plugin didn't declare it, or it's not a plugin made with Rust BG3 template
///
/// # Safety
/// `dll` must be a null terminated utf-16 string
#[export_name = "get_plugin_data"]
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
pub fn get_plugin_data<P: AsRef<Path>>(dll: P) -> Result<Plugin, Box<dyn Error>> {
    let dll = dll
        .as_ref()
        .to_str()
        .ok_or("Failed to convert path to string")?;

    let mut path: Vec<u16> = dll.encode_utf16().collect();
    path.push(b'\0' as u16);

    let path = PCWSTR(path.as_ptr());

    // DONT_RESOLVE_DLL_REFERENCES - Don't call DllMain when loading
    let module = unsafe { LoadLibraryExW(path, None, DONT_RESOLVE_DLL_REFERENCES)? };

    // function name we have to get the plugin details
    let symbol = "PLUGIN_DATA\0";

    // SAFETY:
    // Option<fn()> -> *const T, is safe to cast and dereference as long as Option<fn()> is non-null and points to a valid T
    // https://github.com/rust-lang/unsafe-code-guidelines/issues/440
    let plugin_data =
        unsafe { GetProcAddress(module, PCSTR(symbol.as_ptr())).ok_or("Failed to get address")? }
            as *const Plugin;

    // Safety: If the DLL exported symbol was made from our library and is a plugin, the data should be valid
    let plugin = unsafe { *plugin_data };

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

pub const fn convert_str_to_u16(string: &'static str) -> u16 {
    unwrap_ctx!(parse_u16(string))
}
