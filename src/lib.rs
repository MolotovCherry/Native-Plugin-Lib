mod blob;
mod c;
mod rstr;

use std::{
    fmt::{self, Debug},
    fs::File,
    io::{self, Cursor, Read as _},
    mem,
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt as _};
use eyre::{Context as _, Report, Result};
use konst::{primitive::parse_u16, unwrap_ctx};
use memchr::memchr;
use pelite::pe::{Pe as _, PeFile, exports::GetProcAddress};

pub use crate::rstr::RStr;
use blob::Blob;

#[cfg(not(target_pointer_width = "64"))]
compile_error!("32-bit is not supported");

/// The plugin data version
#[doc(hidden)]
pub const DATA_VERSION: u64 = 1;

/// Plugin details; DATA_VERSION 1
///
/// If you want to identify your own plugin,
/// export a symbol named PLUGIN_DATA containing
/// this data.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Plugin<'a> {
    /// This MUST be set to `DATA_VERSION`
    #[doc(hidden)]
    pub data_ver: u64,
    pub name: RStr<'a>,
    pub author: RStr<'a>,
    pub description: RStr<'a>,
    pub version: Version,
}

impl<'a> Plugin<'a> {
    /// Safely convert from raw data to Plugin
    fn from_raw(data: &'a [u8], to_rstr: impl Fn(u64) -> Option<RStr<'a>>) -> Option<Self> {
        let mut data = Cursor::new(data);

        let version = data.read_u64::<LittleEndian>().ok()?;

        // while these ptrs are guaranteed not null, they are not guaranteed to be valid
        let mut get_rstr = || {
            let ptr = data.read_u64::<LittleEndian>().ok()?;
            to_rstr(ptr)
        };

        let name = get_rstr()?;
        let author = get_rstr()?;
        let description = get_rstr()?;

        let major = data.read_u16::<LittleEndian>().ok()?;
        let minor = data.read_u16::<LittleEndian>().ok()?;
        let patch = data.read_u16::<LittleEndian>().ok()?;

        let this = Self {
            data_ver: version,
            name,
            author,
            description,
            version: Version {
                major,
                minor,
                patch,
            },
        };

        Some(this)
    }
}

impl Debug for Plugin<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Plugin")
            .field("name", &self.name)
            .field("author", &self.author)
            .field("description", &self.description)
            .field("version", &self.version)
            .finish()
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
/// In the crate root file, declare the name, author, and description
/// ```rs
/// declare_plugin!("name", "author", "description");
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
            #[unsafe(no_mangle)]
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
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("{0}")]
    Report(#[from] Report),
    #[error("Data is corrupt")]
    DataCorrupt,
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Pelite(#[from] pelite::Error),
    #[error("Symbol not found in file")]
    SymbolNotFound,
    #[error(
        "Plugin data version is either invalid ({0}), or you need to update to the newest native plugin lib"
    )]
    DataVer(u64),
}

pub struct PluginData {
    plugin: Plugin<'static>,
    #[allow(unused)]
    blob: Blob,
}

impl PluginData {
    pub fn new<P: AsRef<Path>>(dll: P) -> Result<Self, PluginError> {
        let mut file = File::open(dll)?;
        let size = file.metadata()?.len() as usize;

        let mut blob = Blob::new_zeroed(size)?;

        file.read_exact(&mut blob)?;

        let file = PeFile::from_bytes(&blob).context("failed to parse file")?;
        let rva = file
            .get_export("PLUGIN_DATA")
            .ok()
            .and_then(|e| e.symbol())
            .ok_or(PluginError::SymbolNotFound)?;

        let offset = file.rva_to_file_offset(rva)?;

        const _USIZE: usize = size_of::<usize>();
        let _data_ver: [u8; _USIZE] = blob
            .get(offset..offset + _USIZE)
            .ok_or(PluginError::SymbolNotFound)?
            .try_into()
            .unwrap();
        let data_ver = u64::from_ne_bytes(_data_ver);

        // Either the file data is incorrect (corrupted or just flat out wrong)
        // or this library is out of date. It's more likely to be that it's out of date.
        //
        // Do this check first before dereferencing to ensure that dereferenced data is always a valid T

        if !(1..=DATA_VERSION).contains(&data_ver) {
            return Err(PluginError::DataVer(data_ver));
        }

        // Below this line we will handle any future data version changes properly

        let data = Plugin::from_raw(&blob[offset..], |ptr| {
            let rva = file.va_to_rva(ptr).ok()?;
            let offset = file.rva_to_file_offset(rva).ok()?;

            // just keep scanning until \0. If there is one, we have a null terminator
            // this returns if \0 was not found
            let end = memchr(0, blob.get(offset..)?)?;

            // now we have to check for utf8 validity.
            // make sure to include the null terminator as we need it below
            let rstr = {
                let bytes = blob.get(offset..=offset + end)?;
                std::str::from_utf8(bytes).ok()?
            };

            // Safety: String contains a null terminator
            //         checked by memchr
            let rstr = unsafe { RStr::from_str(rstr) };
            Some(rstr)
        })
        .ok_or(PluginError::DataCorrupt)?;

        // Safety: this is packaged along together with blob and dropped at the same time
        let plugin = unsafe { mem::transmute::<Plugin<'_>, Plugin<'static>>(data) };

        let data = Self { blob, plugin };

        Ok(data)
    }

    pub fn plugin(&self) -> Plugin<'_> {
        self.plugin
    }
}

#[doc(hidden)]
pub const fn convert_str_to_u16(string: &'static str) -> u16 {
    unwrap_ctx!(parse_u16(string))
}
