mod c;
mod rstr;

use std::{
    alloc::{self, Layout},
    ffi::c_char,
    fmt::{self, Debug},
    fs::File,
    io::{self, Write},
    path::Path,
    ptr, slice,
    sync::LazyLock,
};

use eyre::{Context, Result};
use konst::{primitive::parse_u16, unwrap_ctx};
use pelite::{
    pe::{Pe as _, PeFile, Va},
    pe64::exports::GetProcAddress,
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

    () => {
        const _: () = {
            #[unsafe(no_mangle)]
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
    data: Plugin<'static>,
}

unsafe impl Send for PluginData {}
unsafe impl Sync for PluginData {}

impl PluginData {
    pub fn data(&self) -> Plugin<'_> {
        self.data
    }
}

impl Drop for PluginData {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.alloc, self.layout);
        }
    }
}

struct WriteableAlloc {
    written: usize,
    size: usize,
    ptr: *mut u8,
}

impl Write for WriteableAlloc {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.written + buf.len() > self.size {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "Buffer overflow"));
        }

        let ptr = unsafe { self.ptr.byte_add(self.written) };
        unsafe {
            ptr::copy(buf.as_ptr(), ptr, buf.len());
        }

        self.written += buf.len();
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        _ = self.write(buf)?;
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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

    let mut file = File::open(dll)?;
    let len = file.metadata()?.len() as usize;
    let gran = (*GRANULARITY as usize).next_power_of_two();

    assert!(gran > 0 && gran.is_power_of_two());
    assert!(len.next_multiple_of(gran) <= isize::MAX as usize);

    let layout = unsafe { Layout::from_size_align_unchecked(len, gran) };
    let alloc = unsafe { alloc::alloc(layout) };

    {
        let mut write = WriteableAlloc {
            written: 0,
            size: len,
            ptr: alloc,
        };

        io::copy(&mut file, &mut write)?;
    }

    let data = unsafe { slice::from_raw_parts(alloc, len) };

    let file = PeFile::from_bytes(&data).context("failed to parse file")?;
    let rva = file
        .get_export("PLUGIN_DATA")
        .context("symbol not found")?
        .symbol()
        .ok_or(pelite::Error::Null)
        .context("failed to find symbol address")?;

    let offset = file.rva_to_file_offset(rva)?;

    let plugin = {
        let ptr = unsafe { alloc.byte_add(offset).cast::<Plugin>() };
        // SAFETY: This is not UB to deref, however touching any RStr is as the internal pointers are wrong
        //         So do not access them until the address has been translated to a file offset
        unsafe { *ptr }
    };

    let va_to_rstr = |ptr: *const c_char| -> Result<RStr> {
        let rva = file.va_to_rva(ptr as Va)?;
        let offset = file.rva_to_file_offset(rva)?;
        let ptr = unsafe { alloc.byte_add(offset).cast() };

        Ok(unsafe { RStr::from_ptr(ptr) })
    };

    let plugin = Plugin {
        data_ver: plugin.data_ver,
        name: va_to_rstr(plugin.name.data)?,
        author: va_to_rstr(plugin.author.data)?,
        description: va_to_rstr(plugin.description.data)?,
        version: plugin.version,
    };

    let data = PluginData {
        alloc,
        layout,
        data: plugin,
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
