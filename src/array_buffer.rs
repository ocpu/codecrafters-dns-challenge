use bytes::buf::{Buf, BufMut, UninitSlice};

use std::{
    fmt, mem, ptr, slice,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct ArrayBuffer {
    read_cursor: usize,
    len: usize,
    max_len: Option<usize>,
    data: *mut Data,
}

struct Data {
    ptr: *mut u8,
    cap: usize,
    refs: AtomicUsize,
}

unsafe impl Send for ArrayBuffer {}
unsafe impl Sync for ArrayBuffer {}

#[inline]
fn ptr_opt<T>(ptr: *mut T) -> Option<*mut T> {
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

#[inline]
fn ptr_opt_ref<'a, T>(ptr: *mut T) -> Option<&'a T> {
    ptr_opt(ptr).and_then(|p| unsafe { p.as_ref() })
}

impl Default for ArrayBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ArrayBuffer {
    pub fn new() -> Self {
        Self {
            read_cursor: 0,
            len: 0,
            max_len: None,
            data: ptr::null::<Data>() as *mut Data,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            read_cursor: 0,
            len: 0,
            max_len: None,
            data: Data::with_capacity(capacity).into_ptr(),
        }
    }

    pub fn with_max_len(mut self, max_len: usize) -> Self {
        self.max_len = Some(max_len);
        self
    }

    pub fn len(&self) -> usize {
        self.max_len.map(|v| v.min(self.len)).unwrap_or(self.len)
    }

    pub fn capacity(&self) -> usize {
        ptr_opt(self.data)
            .map(|ptr| unsafe { ptr.as_ref().unwrap() }.cap)
            .unwrap_or_default()
    }

    pub fn as_slice(&self) -> &[u8] {
        ptr_opt(self.data)
            .map(|data| {
                let len = self.max_len.map(|v| v.min(self.len)).unwrap_or(self.len);
                unsafe { &data.as_ref().unwrap().as_slice()[..len] }
            })
            .unwrap_or(&[])
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        ptr_opt(self.data)
            .map(|data| {
                let len = self.max_len.map(|v| v.min(self.len)).unwrap_or(self.len);
                unsafe { &mut data.as_ref().unwrap().as_slice_mut()[0..len] }
            })
            .unwrap_or(&mut [])
    }

    pub fn set_len(&mut self, new_len: usize) {
        self.len = new_len;
    }

    pub fn clear(&mut self) {
        self.read_cursor = 0;
        self.len = 0;
    }

    fn grow(&mut self, min_new_space: usize) {
        const GROWTH_FACTOR: f64 = 1.5;
        let cap = self.capacity();
        let new_len = usize::max(((cap as f64) * GROWTH_FACTOR) as usize, cap + min_new_space);

        let new_len = self.max_len.map(|l| l.min(new_len)).unwrap_or(new_len);

        if new_len < self.capacity() {
            return;
        }
        let new_data = Data::with_capacity(new_len);

        if let Some(data) = ptr_opt_ref(self.data) {
            new_data.copy_from(data);
            if data.decrement() == 0 {
                drop(unsafe { Box::from_raw(self.data) })
            }
        }

        self.data = new_data.into_ptr();
    }
}

impl Data {
    fn new(ptr: *mut u8, cap: usize) -> Self {
        Self {
            ptr,
            cap,
            refs: AtomicUsize::new(1),
        }
    }

    fn with_capacity(cap: usize) -> Self {
        Self::from_vec(Vec::with_capacity(cap))
    }

    fn from_vec(mut vec: Vec<u8>) -> Self {
        let ptr = vec.as_mut_ptr();
        let cap = vec.capacity();
        mem::forget(vec);
        Self::new(ptr, cap)
    }

    fn copy_from(&self, other: &Data) {
        unsafe {
            self.ptr
                .copy_from_nonoverlapping(other.ptr, self.cap.min(other.cap));
        }
    }

    unsafe fn as_slice(&self) -> &[u8] {
        debug_assert!(!self.ptr.is_null(), "Buffer Data pointer is null");
        slice::from_raw_parts(self.ptr, self.cap)
    }

    unsafe fn as_slice_mut(&self) -> &mut [u8] {
        debug_assert!(!self.ptr.is_null(), "Buffer Data pointer is null");
        slice::from_raw_parts_mut(self.ptr, self.cap)
    }

    unsafe fn as_uninit_slice(&self, start: usize) -> &mut UninitSlice {
        debug_assert!(!self.ptr.is_null(), "Buffer Data pointer is null");
        debug_assert!(start <= self.cap);
        UninitSlice::from_raw_parts_mut(self.ptr.add(start), self.cap - start)
    }

    fn into_ptr(self) -> *mut Data {
        Box::leak(Box::new(self))
    }

    fn decrement(&self) -> usize {
        self.refs.fetch_sub(1, Ordering::SeqCst) - 1
    }
}

impl Drop for ArrayBuffer {
    fn drop(&mut self) {
        if let Some(ptr) = ptr_opt(self.data) {
            if unsafe { ptr.as_ref().unwrap() }.decrement() == 0 {
                drop(unsafe { Box::from_raw(ptr) });
            }
        }
    }
}

impl Drop for Data {
    fn drop(&mut self) {
        drop(unsafe { Vec::from_raw_parts(self.ptr, 0, self.cap) });
    }
}

impl AsRef<[u8]> for ArrayBuffer {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsMut<[u8]> for ArrayBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_slice_mut()
    }
}

impl std::ops::Deref for ArrayBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::ops::DerefMut for ArrayBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl From<&[u8]> for ArrayBuffer {
    fn from(value: &[u8]) -> Self {
        let mut buf = Self::with_capacity(value.len());
        buf.put_slice(value);
        buf
    }
}

unsafe impl BufMut for ArrayBuffer {
    fn remaining_mut(&self) -> usize {
        self.max_len.unwrap_or(usize::MAX).saturating_sub(self.len)
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        assert!(
            self.len + cnt < self.max_len.unwrap_or(usize::MAX),
            "Cursor beyond max len"
        );
        self.len += cnt;
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        if self.len >= self.capacity() {
            self.grow(usize::max(64, self.len - self.capacity()));
        }
        ptr_opt_ref(self.data)
            .map(|data| unsafe { data.as_uninit_slice(self.len) })
            .expect("Data is null")
    }
}

impl Buf for ArrayBuffer {
    fn remaining(&self) -> usize {
        self.len - self.read_cursor
    }

    fn advance(&mut self, cnt: usize) {
        assert!(self.read_cursor + cnt < self.len, "Cursor beyond len");
        self.read_cursor += cnt;
    }

    fn chunk(&self) -> &[u8] {
        &self.as_slice()[self.read_cursor..]
    }
}

impl fmt::Debug for ArrayBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

const LINE_ITEM_COUNT: usize = 16;
impl fmt::Binary for ArrayBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = self.as_slice();
        loop {
            let slice = &buffer[..usize::min(LINE_ITEM_COUNT, buffer.len())];
            buffer = &buffer[slice.len()..];

            for i in 0..LINE_ITEM_COUNT {
                if let Some(byte) = slice.get(i) {
                    if *byte < 16 {
                        write!(f, "0")?;
                    }
                    write!(f, "{byte:x?} ")?;
                } else {
                    write!(f, "   ")?;
                }
            }

            for i in 0..LINE_ITEM_COUNT {
                if let Some(byte) = slice.get(i) {
                    if byte.is_ascii_alphanumeric() || *byte == b'-' {
                        write!(f, "{}", *byte as char)?;
                    } else {
                        write!(f, ".")?;
                    }
                } else {
                    write!(f, " ")?;
                }
            }
            write!(f, "\n")?;
            if buffer.is_empty() {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocation_with_capacity() {
        let buf = ArrayBuffer::with_capacity(10);
        let data_ptr = buf.data;
        drop(buf);
        assert_ne!(10, unsafe { data_ptr.as_ref().unwrap().cap });
    }

    #[test]
    fn allocation_with_grow() {
        let mut buf = ArrayBuffer::new();
        buf.grow(10);
        let data_ptr = buf.data;
        drop(buf);
        assert_ne!(10, unsafe { data_ptr.as_ref().unwrap().cap });
    }

    #[test]
    fn allocation_with_capacity_and_grow() {
        let mut buf = ArrayBuffer::with_capacity(10);
        let data_ptr = buf.data;
        buf.grow(10);
        assert_ne!(10, unsafe { data_ptr.as_ref().unwrap().cap });
        let data_ptr = buf.data;
        drop(buf);
        assert_ne!(20, unsafe { data_ptr.as_ref().unwrap().cap });
    }

    #[test]
    fn can_put_data() {
        let mut buf = ArrayBuffer::with_capacity(10);
        buf.put_u8(1);
        buf.put_u16(6);
        assert_eq!(&[1, 0, 6], buf.as_slice());
    }

    #[test]
    fn data_is_copied_when_growing() {
        let mut buf = ArrayBuffer::with_capacity(10);
        buf.put_u32(0x00010001);
        buf.put_u32(0x00010001);
        buf.put_u16(1);
        buf.put_u32(0x00010001);
        assert_eq!(&[0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1, 0, 1], buf.as_slice());
    }
}
