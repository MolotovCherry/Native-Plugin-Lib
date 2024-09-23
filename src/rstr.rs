use std::{
    ffi::c_char,
    fmt::{self, Display},
    marker::PhantomData,
    ops::Deref,
    slice, str,
};

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

unsafe impl Send for RStr<'_> {}
unsafe impl Sync for RStr<'_> {}

impl<'a> RStr<'a> {
    pub(crate) const fn from_str(data: &'a str) -> Self {
        Self {
            data: data.as_ptr().cast(),
            len: data.len(),
            _marker: PhantomData::<&'a ()>,
        }
    }

    pub fn as_str(&self) -> &'a str {
        let slice = unsafe { slice::from_raw_parts(self.data.cast::<u8>(), self.len) };
        unsafe { str::from_utf8_unchecked(slice) }
    }
}

impl Display for RStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<'a> Deref for RStr<'a> {
    type Target = str;

    fn deref(&self) -> &'a Self::Target {
        self.as_str()
    }
}
