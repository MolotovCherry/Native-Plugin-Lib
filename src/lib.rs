mod c;
mod rstr;

use std::{
    alloc::{self, Layout},
    ffi::c_char,
    fs,
    path::Path,
    ptr, slice,
    sync::LazyLock,
};

use eyre::{bail, Context, Result};
use konst::{primitive::parse_u16, unwrap_ctx};
use pelite::{
    pe::{Pe as _, PeFile, Rva},
    pe64::{exports::GetProcAddress, headers::SectionHeader},
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

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

pub struct PluginData {
    alloc: *mut u8,
    layout: Layout,
    offset: usize,
    section: SectionHeader,
    image_base: u64,
}

impl PluginData {
    pub fn data(&self) -> Plugin<'_> {
        let ptr = unsafe { self.alloc.byte_add(self.offset).cast::<Plugin>() };
        let plugin = unsafe { &*ptr };

        let plugin = Plugin {
            data_ver: plugin.data_ver,
            name: self.get_rstr(plugin.name.data),
            author: self.get_rstr(plugin.author.data),
            description: self.get_rstr(plugin.description.data),
            version: plugin.version,
        };

        plugin
    }

    fn get_rstr(&self, ptr: *const c_char) -> RStr<'_> {
        let rva = (ptr as usize - self.image_base as usize) as Rva;
        let section_offset = rva - self.section.VirtualAddress;
        let file_offset = (section_offset + self.section.PointerToRawData) as usize;

        let ptr = unsafe { self.alloc.byte_add(file_offset) };

        unsafe { RStr::from_ptr(ptr.cast()) }
    }
}

impl Drop for PluginData {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.alloc, self.layout);
        }
    }
}

/// Rust function to get plugin data from from a plugin dll
pub fn get_plugin_data<P: AsRef<Path>>(dll: P) -> Result<PluginData> {
    static GRANULARITY: LazyLock<u32> = LazyLock::new(|| {
        let mut info = SYSTEM_INFO::default();
        unsafe {
            GetSystemInfo(&mut info);
        }

        info.dwAllocationGranularity
    });

    let data = fs::read(dll)?;
    let len = data.len();

    let layout = unsafe { Layout::from_size_align_unchecked(len, *GRANULARITY as usize) };
    let alloc = unsafe { alloc::alloc(layout) };
    unsafe {
        ptr::copy_nonoverlapping(data.as_ptr(), alloc, len);
    }

    let slice = unsafe { slice::from_raw_parts(alloc, len) };

    let file = PeFile::from_bytes(slice).context("failed to parse file")?;
    let rva = file
        .get_export("PLUGIN_DATA")?
        .symbol()
        .ok_or(pelite::Error::Null)
        .context("failed to find symbol address")?;

    let offset = file.rva_to_file_offset(rva)?;
    let image_base = file.optional_header().ImageBase;
    let mut section = None;

    for it in file.section_headers() {
        let section_name = std::str::from_utf8(it.name_bytes())?;
        if section_name == ".rdata" {
            section = Some(*it);
            break;
        }
    }

    let Some(section) = section else {
        bail!(".rdata section not found");
    };

    let data = PluginData {
        alloc,
        layout,
        offset,
        section,
        image_base,
    };

    Ok(data)
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
