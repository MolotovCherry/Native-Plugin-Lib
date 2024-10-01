mod c;
mod rstr;

use std::{error::Error, iter, path::Path, sync::Arc};

use konst::{primitive::parse_u16, unwrap_ctx};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES},
    },
};

pub use crate::rstr::RStr;

/// The plugin data version
#[doc(hidden)]
pub const DATA_VERSION: usize = 1;

/// Plugin details; DATA_VERSION 1
///
/// If you want to identify your own plugin,
/// export a symbol named PLUGIN_DATA containing
/// this data.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Plugin<'a> {
    /// This MUST be set to `DATA_VERSION`
    #[doc(hidden)]
    pub data_ver: usize,
    pub name: RStr<'a>,
    pub author: RStr<'a>,
    pub description: RStr<'a>,
    pub version: Version,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

/// Define a plugin's name, author, and description
///
/// In the crate root file, declare the name, author, and description
/// ```rs
/// declare_plugin!("name", "author", "description");
/// ```
/// You can also use it without to use the Cargo.toml name, authors, and description fields
/// ```rs
/// declare_plugin!();
/// ```
///
/// env!() macro is also possible to use by itself if you need more customization
/// ```rs
/// declare_plugin!(env!("CARGO_PKG_NAME"), "author", "description");
/// ```
///
/// The strings must not contain any null bytes in them
#[macro_export]
macro_rules! declare_plugin {
    ($name:expr, $author:expr, $desc:expr) => {
        const _: () = {
            #[no_mangle]
            static PLUGIN_DATA: $crate::Plugin<'static> = $crate::Plugin {
                data_ver: $crate::DATA_VERSION,
                name: unsafe { $crate::RStr::from_str(concat!($name, "\0")) },
                author: unsafe { $crate::RStr::from_str(concat!($author, "\0")) },
                description: unsafe { $crate::RStr::from_str(concat!($desc, "\0")) },
                version: $crate::Version {
                    major: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MAJOR")),
                    minor: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MINOR")),
                    patch: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_PATCH")),
                },
            };
        };
    };

    () => {
        const _: () = {
            #[no_mangle]
            static PLUGIN_DATA: $crate::Plugin<'static> = $crate::Plugin {
                data_ver: $crate::DATA_VERSION,
                name: unsafe { $crate::RStr::from_str(concat!(env!("CARGO_PKG_NAME"), "\0")) },
                author: unsafe { $crate::RStr::from_str(concat!(env!("CARGO_PKG_AUTHORS"), "\0")) },
                description: unsafe {
                    $crate::RStr::from_str(concat!(env!("CARGO_PKG_DESCRIPTION"), "\0"))
                },
                version: $crate::Version {
                    major: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MAJOR")),
                    minor: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MINOR")),
                    patch: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_PATCH")),
                },
            };
        };
    };
}

#[derive(Debug, Clone)]
pub struct PluginGuard<'a> {
    module: Arc<Module<'a>>,
}

impl PluginGuard<'_> {
    pub fn data(&self) -> Plugin<'_> {
        self.module.data()
    }
}

// Drop wrapper to handle the module reference
#[derive(Debug)]
struct Module<'a>(HMODULE, Plugin<'a>);

unsafe impl Send for Module<'_> {}
unsafe impl Sync for Module<'_> {}

impl<'a> Module<'a> {
    fn new(value: HMODULE, plugin: Plugin<'a>) -> Self {
        Self(value, plugin)
    }

    fn data(&self) -> Plugin<'_> {
        self.1
    }
}

impl Drop for Module<'_> {
    fn drop(&mut self) {
        _ = unsafe { FreeLibrary(self.0) };
    }
}

/// Rust function to get plugin data from from a plugin dll
pub fn get_plugin_data<'a, P: AsRef<Path>>(dll: P) -> Result<PluginGuard<'a>, Box<dyn Error>> {
    let dll = dll
        .as_ref()
        .to_str()
        .ok_or("Failed to convert path to string")?;

    let path = dll.encode_utf16().chain(iter::once(0)).collect::<Vec<_>>();
    let path = PCWSTR(path.as_ptr());

    // DONT_RESOLVE_DLL_REFERENCES - Don't call DllMain when loading
    let module = unsafe { LoadLibraryExW(path, None, DONT_RESOLVE_DLL_REFERENCES)? };

    // function name we have to get the plugin details
    let symbol = "PLUGIN_DATA\0";

    // SAFETY:
    // Option<fn()> -> *const T, is safe to cast and dereference as long as Option<fn()> is non-null and points to a valid T
    // https://github.com/rust-lang/unsafe-code-guidelines/issues/440
    let plugin_data = unsafe { GetProcAddress(module, PCSTR(symbol.as_ptr())) };
    let plugin_data = plugin_data.ok_or("plugin data unexpectedly null")? as *const Plugin<'a>;

    // Safety: If the DLL exported symbol was made from our library and is a plugin, the data should be valid
    let plugin = unsafe { *plugin_data };

    let guard: PluginGuard<'a> = PluginGuard {
        module: Arc::new(Module::new(module, plugin)),
    };

    Ok(guard)
}

/// Convert static string to compile time array
#[doc(hidden)]
pub const fn convert_str<const N: usize>(string: &'static str) -> [u8; N] {
    assert!(
        string.len() < N,
        "String len must be < total available space"
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

#[doc(hidden)]
pub const fn convert_str_to_u16(string: &'static str) -> u16 {
    unwrap_ctx!(parse_u16(string))
}
