use super::super::MemReader;
use failure::{format_err, Error};
use std::ffi::CStr;
use std::mem::size_of;
use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{DWORD, FALSE, HMODULE, LPVOID, MAX_PATH, TRUE};
use winapi::shared::ntdef::{HANDLE, NULL};
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi;
use winapi::um::processthreadsapi;
use winapi::um::psapi;
use winapi::um::winnt::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

pub struct Process {
    handle: HANDLE,
    name: String,

    pub base_addr: LPVOID,
    pub base_size: usize,
    entry_point: LPVOID,

    base_contents: Box<Vec<u8>>,
}

impl Process {
    pub fn open_by_pid(pid: DWORD) -> Option<Process> {
        // This procedure is adopted from:
        //   https://docs.microsoft.com/en-us/windows/win32/psapi/enumerating-all-processes
        if pid == 0 {
            return None;
        }

        unsafe {
            let proc_handle = processthreadsapi::OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                FALSE,
                pid,
            );

            if proc_handle == NULL {
                return None;
            }

            //
            let mut module: HMODULE = std::ptr::null_mut();
            let mut cb_needed: DWORD = 0;
            let success = psapi::EnumProcessModules(
                proc_handle,
                &mut module as *mut HMODULE,
                size_of::<HMODULE>() as DWORD,
                &mut cb_needed as *mut DWORD,
            );

            // From here on, handle will automatically close.
            let mut proc = Process {
                handle: proc_handle,
                name: "".to_string(),
                base_addr: NULL,
                base_size: 0,
                entry_point: NULL,
                base_contents: Box::new(Vec::new()),
            };

            if success == FALSE {
                return None;
            }

            // Read process name
            let mut raw_name: Vec<i8> = vec![0; MAX_PATH];
            psapi::GetModuleBaseNameA(
                proc_handle,
                module,
                raw_name.as_mut_ptr(),
                raw_name.len() as DWORD,
            );
            let name = CStr::from_ptr(raw_name.as_ptr()).to_string_lossy();
            proc.name = name.into_owned();

            let mut info: psapi::MODULEINFO = Default::default();

            let success = psapi::GetModuleInformation(
                proc_handle,
                module,
                &mut info as *mut psapi::MODULEINFO,
                size_of::<psapi::MODULEINFO>() as DWORD,
            );
            if success == FALSE {
                return None;
            }

            proc.base_addr = info.lpBaseOfDll;
            proc.base_size = info.SizeOfImage as usize;
            proc.entry_point = info.EntryPoint;

            Some(proc)
        }
    }

    pub fn open_by_name(name: &str) -> Option<Process> {
        unsafe {
            let mut procs: Vec<DWORD> = vec![0; 1024];
            let mut cb_needed: DWORD = 0;

            psapi::EnumProcesses(
                procs.as_mut_ptr(),
                (procs.len() * size_of::<DWORD>()) as u32,
                &mut cb_needed as *mut DWORD,
            );

            let num_procs = cb_needed as usize / size_of::<DWORD>();

            for i in 0..num_procs {
                let process = match Process::open_by_pid(procs[i]) {
                    Some(p) => p,
                    None => continue,
                };

                if process.name == name {
                    return Some(process);
                }
            }
            None
        }
    }

    pub fn read_memory(&self, buf: &mut [u8], addr: LPVOID, len: usize) -> usize {
        let read_len = if buf.len() < len { buf.len() } else { len };

        let mut bytes_read: usize = 0;

        let success = unsafe {
            memoryapi::ReadProcessMemory(
                self.handle,
                addr,
                buf.as_mut_ptr() as LPVOID,
                read_len as SIZE_T,
                &mut bytes_read as *mut SIZE_T,
            )
        };
        if success == TRUE {
            bytes_read
        } else {
            0
        }
    }

    /// Load a cached copy of the process' BaseModule.
    ///
    /// This allows for much faster resolving of `Signature`s
    pub fn load_base(&mut self) -> Result<(), Error> {
        let mut buf = Box::new(vec![0; self.base_size]);

        let read_size = self.read_memory(&mut buf, self.base_addr, self.base_size);
        if read_size != self.base_size {
            return Err(format_err!(
                "only read {} bytes of {}.",
                read_size,
                self.base_size
            ));
        }
        self.base_contents = buf;
        Ok(())
    }

    /// Unloads the cached copy, if any, of the process' BaseModule.
    pub fn unload_base(&mut self) {
        self.base_contents = Box::new(Vec::new());
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

impl MemReader for Process {
    fn read(&self, buf: &mut [u8], addr: u64, len: usize) -> usize {
        let base_addr = self.base_addr as u64;
        if addr >= base_addr {
            let start_index = (addr - base_addr) as usize;
            let end_index = start_index + len - 1;
            if end_index < self.base_contents.len() {
                buf.copy_from_slice(&self.base_contents[start_index..=end_index]);
                return len;
            }
        }
        self.read_memory(buf, addr as LPVOID, len)
    }
}
