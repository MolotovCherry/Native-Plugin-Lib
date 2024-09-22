mod c;

use std::{error::Error, iter, path::Path, sync::Arc};

use konst::{primitive::parse_u16, unwrap_ctx};
use windows::{
    core::{PCSTR, PCWSTR},
    Win32::{
        Foundation::{FreeLibrary, HMODULE},
        System::LibraryLoader::{GetProcAddress, LoadLibraryExW, DONT_RESOLVE_DLL_REFERENCES},
    },
};

pub use abi_stable::std_types::RStr;

/// The plugin data version
/// This is used in C interface. Rust users can ignore it
pub const DATA_VERSION: usize = 1;

/// Plugin details
/// cbindgen:ignore
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Plugin<'a> {
    data_ver: usize,
    pub name: RStr<'a>,
    pub author: RStr<'a>,
    pub description: RStr<'a>,
    pub version: Version,
}

impl Plugin<'_> {
    pub const fn new(
        name: &'static str,
        author: &'static str,
        description: &'static str,
        version: Version,
    ) -> Self {
        Self {
            data_ver: DATA_VERSION,
            name: RStr::from_str(name),
            author: RStr::from_str(author),
            description: RStr::from_str(description),
            version,
        }
    }
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
/// The strings must not contain any null bytes in them
#[macro_export]
macro_rules! declare_plugin {
    ($name:literal, $author:literal, $desc:literal) => {
        #[no_mangle]
        static PLUGIN_DATA: $crate::Plugin<'static> = $crate::Plugin::new(
            $name,
            $author,
            $desc,
            $crate::Version {
                major: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MAJOR")),
                minor: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_MINOR")),
                patch: $crate::convert_str_to_u16(env!("CARGO_PKG_VERSION_PATCH")),
            },
        );
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

pub const fn convert_str_to_u16(string: &'static str) -> u16 {
    unwrap_ctx!(parse_u16(string))
}
