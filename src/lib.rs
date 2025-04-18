mod blob;
mod c;
mod rstr;

use std::{
    array::TryFromSliceError,
    ffi::c_char,
    fmt::{self, Debug},
    fs::File,
    io::{self, Read as _},
    path::Path,
    ptr::NonNull,
};

use blob::Blob;
use eyre::{Context as _, Report, Result};
use konst::{primitive::parse_u16, unwrap_ctx};
use pelite::{
    pe::{Pe as _, PeFile, Va},
    pe64::exports::GetProcAddress,
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
#[derive(Copy, Clone)]
pub struct Plugin<'a> {
    /// This MUST be set to `DATA_VERSION`
    #[doc(hidden)]
    pub data_ver: usize,
    pub name: RStr<'a>,
    pub author: RStr<'a>,
    pub description: RStr<'a>,
    pub version: Version,
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

pub struct PluginData {
    #[allow(unused)]
    blob: Blob,
    data: Plugin<'static>,
}

impl PluginData {
    pub fn data(&self) -> Plugin<'_> {
        self.data
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("{0}")]
    Report(#[from] Report),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Pelite(#[from] pelite::Error),
    #[error("Symbol not found in file")]
    SymbolNotFound,
    #[error("{0}")]
    SliceErr(#[from] TryFromSliceError),
    #[error(
        "Plugin data version is either invalid ({0}), or you need to update to the newest native plugin lib"
    )]
    DataVer(usize),
}

/// Rust function to get plugin data from from a plugin dll
pub fn get_plugin_data<P: AsRef<Path>>(dll: P) -> Result<PluginData, PluginError> {
    let mut file = File::open(dll)?;
    let size = file.metadata()?.len() as usize;

    let mut blob = Blob::new_zeroed(size)?;

    file.read_exact(&mut blob)?;

    let file = PeFile::from_bytes(&blob).context("failed to parse file")?;
    let rva = file
        .get_export("PLUGIN_DATA")
        .map_err(|_| PluginError::SymbolNotFound)?
        .symbol()
        .ok_or(pelite::Error::Null)
        .map_err(|_| PluginError::SymbolNotFound)?;

    let offset = file.rva_to_file_offset(rva)?;

    const _USIZE: usize = size_of::<usize>();
    let _data_ver: [u8; _USIZE] = blob[offset..offset + _USIZE].try_into()?;
    let data_ver = usize::from_ne_bytes(_data_ver);

    // Either the file data is incorrect (corrupted or just flat out wrong)
    // or this library is out of date. It's more likely to be that it's out of date.
    //
    // Do this check first before dereferencing to ensure that dereferenced data is always a valid T

    let versions = const { gen_versions() };
    if !versions.contains(&data_ver) {
        return Err(PluginError::DataVer(data_ver));
    }

    // Below this line we will handle any future data version changes properly

    let data = {
        let ptr = blob[offset..].as_ptr().cast::<Plugin>();
        assert!(ptr.is_aligned());
        // SAFETY: We only get here until after we found the exported symbol exists. At this point we have to
        //         trust that the dll symbol has the correct data
        //         This is not UB to deref, however reading any RStr is as the internal pointers are wrong
        //         So do not access them until the address has been translated to a file offset
        unsafe { *ptr }
    };

    let va_to_rstr = |ptr: NonNull<c_char>| -> Result<RStr> {
        let ptr = ptr.as_ptr();

        let rva = file.va_to_rva(ptr as Va)?;
        let offset = file.rva_to_file_offset(rva)?;
        let ptr = blob[offset..].as_ptr().cast::<c_char>();

        Ok(unsafe { RStr::from_ptr(ptr) })
    };

    let data = Plugin {
        data_ver: data.data_ver,
        name: va_to_rstr(data.name.ptr)?,
        author: va_to_rstr(data.author.ptr)?,
        description: va_to_rstr(data.description.ptr)?,
        version: data.version,
    };

    let data = PluginData { blob, data };

    Ok(data)
}

#[doc(hidden)]
pub const fn convert_str_to_u16(string: &'static str) -> u16 {
    unwrap_ctx!(parse_u16(string))
}

const fn gen_versions() -> [usize; DATA_VERSION] {
    let mut arr = [0; DATA_VERSION];
    let mut i = 0;
    while i < DATA_VERSION {
        arr[i] = i + 1;
        i += 1;
    }

    arr
}
