use std::{
    alloc::{self, Layout},
    ops::{Deref, DerefMut},
    ptr::NonNull,
    slice,
    sync::LazyLock,
};

use eyre::{OptionExt as _, Result};
use stable_deref_trait::StableDeref;
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};

pub struct Blob {
    layout: Layout,
    data: NonNull<u8>,
}

// Safety: Blob is heap allocated
unsafe impl StableDeref for Blob {}

impl Blob {
    pub fn new_zeroed(size: usize) -> Result<Self> {
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
        // SAFETY: Data exists, is valid, has been initialized by alloc zeroed
        unsafe { slice::from_raw_parts(self.data.as_ptr(), self.layout.size()) }
    }
}

impl DerefMut for Blob {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Data exists, is valid, has been initialized by alloc zeroed
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
