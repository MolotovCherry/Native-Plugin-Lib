use std::{
    ffi::{c_char, CStr},
    fmt::{self, Debug, Display},
    marker::PhantomData,
    ops::Deref,
    ptr::NonNull,
    str,
};

/// utf8 null terminated string.
///
/// # Safety
/// This points to a valid utf-8 string
/// Contains no internal nulls
/// Contains a null terminator
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct RStr<'a> {
    pub(crate) ptr: NonNull<c_char>,
    _marker: PhantomData<&'a str>,
}

unsafe impl Send for RStr<'_> {}
unsafe impl Sync for RStr<'_> {}

impl<'a> RStr<'a> {
    /// # Safety
    /// string must contain null terminator
    #[doc(hidden)]
    pub const unsafe fn from_str(data: &'static str) -> Self {
        let ptr = data.as_ptr().cast::<c_char>();

        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut()) },
            _marker: PhantomData,
        }
    }

    /// # Safety
    /// Ptr must be non-null and have a null terminator
    pub(crate) unsafe fn from_ptr(ptr: *const c_char) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr.cast_mut()) },
            _marker: PhantomData,
        }
    }

    fn as_str(&self) -> &'a str {
        let cstr = unsafe { CStr::from_ptr(self.ptr.as_ptr()) };
        unsafe { str::from_utf8_unchecked(cstr.to_bytes()) }
    }
}

impl Debug for RStr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:?}", self.as_str()))
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
