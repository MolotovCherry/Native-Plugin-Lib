use core::str;
use std::{ffi::c_char, marker::PhantomData, ops::Deref, slice};

/// A ffi safe rust string.
///
/// # Safety
/// This points to a valid utf-8 string
/// This does not contain a null terminator
/// This is only valid for reads up to `len`
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RStr<'a> {
    data: *const c_char,
    len: usize,
    _marker: PhantomData<&'a ()>,
}

impl<'a> RStr<'a> {
    pub(crate) const fn from_str(data: &'a str) -> Self {
        Self {
            data: data.as_ptr().cast(),
            len: data.len(),
            _marker: PhantomData::<&'a ()>,
        }
    }
}

impl<'a> Deref for RStr<'a> {
    type Target = str;

    fn deref(&self) -> &'a Self::Target {
        let slice = unsafe { slice::from_raw_parts(self.data.cast::<u8>(), self.len) };
        unsafe { str::from_utf8_unchecked(slice) }
    }
}
