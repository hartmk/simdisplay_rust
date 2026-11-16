//! Read-only Windows named mappings. Opening a receiver never creates game pages.
#![cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::Memory::{
        FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile,
    },
};

pub struct ReadMapping {
    handle: HANDLE,
    view: MEMORY_MAPPED_VIEW_ADDRESS,
    length: usize,
}
impl ReadMapping {
    pub fn open(name: &str, length: usize) -> Result<Option<Self>, String> {
        let wide: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        // SAFETY: the name is NUL-terminated and handles/views are owned below.
        let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, 0, wide.as_ptr()) };
        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            return if matches!(error.raw_os_error(), Some(2 | 3)) {
                Ok(None)
            } else {
                Err(format!("Shared Memory {name}: {error}"))
            };
        }
        let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, length) };
        if view.Value.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe {
                CloseHandle(handle);
            }
            return Err(format!("Shared Memory {name} ({length} bytes): {error}"));
        }
        Ok(Some(Self {
            handle,
            view,
            length,
        }))
    }
    pub fn copy(&self, sequenced: bool) -> Option<Vec<u8>> {
        if sequenced && self.length < 4 {
            return None;
        }
        let pointer = self.view.Value.cast::<u8>();
        for _ in 0..3 {
            // AC updates these pages asynchronously. Compare the packet counter
            // around a volatile copy and retry if it changes during the read.
            let before = if sequenced {
                unsafe { std::ptr::read_volatile(pointer.cast::<i32>()) }
            } else {
                0
            };
            let data: Vec<u8> = (0..self.length)
                .map(|i| unsafe { std::ptr::read_volatile(pointer.add(i)) })
                .collect();
            std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
            let after = if sequenced {
                unsafe { std::ptr::read_volatile(pointer.cast::<i32>()) }
            } else {
                0
            };
            if before == after {
                return Some(data);
            }
        }
        None
    }
}
impl Drop for ReadMapping {
    fn drop(&mut self) {
        // SAFETY: both resources were acquired together and are released once.
        unsafe {
            UnmapViewOfFile(self.view);
            CloseHandle(self.handle);
        }
    }
}
