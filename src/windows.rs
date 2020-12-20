use libc;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::i32;
use std::io::{Error, Result};

pub struct Semaphore {
    handle: libc::HANDLE,
}

pub const WAIT_FAILED: libc::DWORD = 0xFFFFFFFF;
pub const WAIT_TIMEOUT: libc::DWORD = 0x00000102;

extern "system" {
    fn CreateSemaphoreW(
        lpSemaphoreAttributes: libc::LPSECURITY_ATTRIBUTES,
        lInitialCount: libc::LONG,
        lMaximumCount: libc::LONG,
        lpName: libc::LPCWSTR,
    ) -> libc::HANDLE;
    fn ReleaseSemaphore(
        hSemaphore: libc::HANDLE,
        lReleaseCount: libc::LONG,
        lpPreviousCount: *mut libc::LONG,
    ) -> libc::BOOL;
}

impl Semaphore {
    /// Get value hash
    fn hash<T: Hash>(value: &T) -> u64 {
        let mut h = DefaultHasher::new();
        value.hash(&mut h);
        h.finish()
    }

    pub unsafe fn new(name: &str, cnt: usize) -> Result<Semaphore> {
        let name = format!(
            r"Global\{}-{}",
            name.replace(r"\", ""),
            Semaphore::hash::<_>(&(name, "ipc-rs"))
        );
        let mut name = name.bytes().map(|b| b as u16).collect::<Vec<u16>>();
        name.push(0);
        let handle = CreateSemaphoreW(
            std::ptr::null_mut(),
            cnt as libc::LONG,
            i32::MAX as libc::LONG,
            name.as_ptr(),
        );
        if handle.is_null() {
            Err(Error::last_os_error())
        } else {
            Ok(Semaphore { handle })
        }
    }

    pub unsafe fn wait(&self) {
        match libc::WaitForSingleObject(self.handle, libc::INFINITE) {
            libc::WAIT_OBJECT_0 => {}
            WAIT_FAILED => panic!("failed to wait: {}", Error::last_os_error()),
            n => panic!("bad wait(): {}/{}", n, Error::last_os_error()),
        }
    }

    pub unsafe fn try_wait(&self) -> bool {
        match libc::WaitForSingleObject(self.handle, 0) {
            libc::WAIT_OBJECT_0 => true,
            WAIT_TIMEOUT => false,
            WAIT_FAILED => panic!("failed to wait: {}", Error::last_os_error()),
            n => panic!("bad wait(): {}/{}", n, Error::last_os_error()),
        }
    }

    pub unsafe fn post(&self) {
        if let 0 = ReleaseSemaphore(self.handle, 1, std::ptr::null_mut()) {
            panic!("failed to release semaphore: {}", Error::last_os_error())
        }
    }
}

unsafe impl Send for Semaphore {}
unsafe impl Sync for Semaphore {}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            libc::CloseHandle(self.handle);
        }
    }
}
