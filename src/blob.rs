use std::{
    alloc::{self, Layout},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    sync::LazyLock,
};

use eyre::{OptionExt as _, Result};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

pub struct Blob {
    layout: Layout,
    data: NonNull<u8>,
}

impl Blob {
    pub fn new(size: usize) -> Result<Self> {
        static GRANULARITY: LazyLock<u32> = LazyLock::new(|| {
            let mut info = SYSTEM_INFO::default();
            unsafe {
                GetSystemInfo(&mut info);
            }

            info.dwAllocationGranularity
        });

        let layout = Layout::from_size_align(size, *GRANULARITY as usize)?;

        let ptr = unsafe { alloc::alloc_zeroed(layout) };

        let this = Self {
            layout,
            data: NonNull::new(ptr).ok_or_eyre("failed to alloc blob")?,
        };

        Ok(this)
    }
}

impl AsRef<[u8]> for Blob {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl Deref for Blob {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.layout.size()) }
    }
}

impl DerefMut for Blob {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { slice::from_raw_parts_mut(self.data.as_ptr(), self.layout.size()) }
    }
}

impl Drop for Blob {
    fn drop(&mut self) {
        unsafe {
            alloc::dealloc(self.data.as_ptr(), self.layout);
        }
    }
}
