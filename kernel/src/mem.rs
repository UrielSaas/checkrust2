//! Data structure for passing application memory to the kernel.

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ptr::Unique;
use core::slice;

use crate::callback::AppId;

/// Type for specifying an AppSlice is hidden from the kernel.
#[derive(Debug)]
pub struct Private;

/// Type for specifying an AppSlice is shared with the kernel.
#[derive(Debug)]
pub struct Shared;

/// Base type for an AppSlice that holds the raw pointer to the memory region
/// the app shared with the kernel.
pub struct AppPtr<'ker, L, T> {
    ptr: Unique<T>,
    process: AppId<'ker>,
    _phantom: PhantomData<L>,
}

impl<'ker, L, T> AppPtr<'ker, L, T> {
    unsafe fn new(ptr: *mut T, appid: AppId<'ker>) -> AppPtr<'ker, L, T> {
        AppPtr {
            ptr: Unique::new_unchecked(ptr),
            process: appid,
            _phantom: PhantomData,
        }
    }
}

impl<L, T> Deref for AppPtr<'_, L, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}

impl<L, T> DerefMut for AppPtr<'_, L, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}

impl<L, T> Drop for AppPtr<'_, L, T> {
    fn drop(&mut self) {
        self.process
            .kernel
            .process_map_or((), self.process.idx(), |process| unsafe {
                process.free(self.ptr.as_ptr() as *mut u8)
            })
    }
}

/// Buffer of memory shared from an app to the kernel.
///
/// This is the type created after an app calls the `allow` syscall.
pub struct AppSlice<'ker, L, T> {
    ptr: AppPtr<'ker, L, T>,
    len: usize,
}

impl<'ker, L, T> AppSlice<'ker, L, T> {
    crate fn new(ptr: *mut T, len: usize, appid: AppId<'ker>) -> AppSlice<'ker, L, T> {
        unsafe {
            AppSlice {
                ptr: AppPtr::new(ptr, appid),
                len: len,
            }
        }
    }

    /// Number of bytes in the `AppSlice`.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get the raw pointer to the buffer. This will be a pointer inside of the
    /// app's memory region.
    pub fn ptr(&self) -> *const T {
        self.ptr.ptr.as_ptr()
    }

    /// Provide access to one app's AppSlice to another app. This is used for
    /// IPC.
    crate unsafe fn expose_to(&self, appid: AppId<'ker>) -> bool {
        if appid.idx() != self.ptr.process.idx() {
            self.ptr
                .process
                .kernel
                .process_map_or(false, appid.idx(), |process| {
                    process
                        .add_mpu_region(self.ptr() as *const u8, self.len(), self.len())
                        .is_some()
                })
        } else {
            false
        }
    }

    pub fn iter(&self) -> slice::Iter<T> {
        self.as_ref().iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<T> {
        self.as_mut().iter_mut()
    }

    pub fn chunks(&self, size: usize) -> slice::Chunks<T> {
        self.as_ref().chunks(size)
    }

    pub fn chunks_mut(&mut self, size: usize) -> slice::ChunksMut<T> {
        self.as_mut().chunks_mut(size)
    }
}

impl<L, T> AsRef<[T]> for AppSlice<'_, L, T> {
    fn as_ref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr.ptr.as_ref(), self.len) }
    }
}

impl<L, T> AsMut<[T]> for AppSlice<'_, L, T> {
    fn as_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr.ptr.as_mut(), self.len) }
    }
}
