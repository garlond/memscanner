use super::test::TestMemReader;
use super::ArrayConfig;
use super::MemReader;
use failure::{format_err, Error};
use num_traits::FromPrimitive;

use std::mem::size_of_val;

pub fn new_mem_cache(config: &ArrayConfig) -> TestMemReader {
    let mut data = Vec::with_capacity(config.element_size as usize);
    data.resize_with(config.element_size as usize, Default::default);
    TestMemReader {
        mem: data,
        start_addr: 0x0,
    }
}

pub fn update_mem_cache(
    mem: &dyn MemReader,
    cached_mem: &mut TestMemReader,
    base_addr: u64,
    len: u64,
) -> Result<(), Error> {
    let read_len = mem.read(&mut cached_mem.mem, base_addr, len as usize);
    if read_len != len as usize {
        return Err(format_err!("could not read {} bytes", len));
    }
    cached_mem.start_addr = base_addr;
    Ok(())
}

pub fn get_array_base_addr(
    config: &ArrayConfig,
    base_addr: u64,
    index: usize,
    mem: &dyn MemReader,
) -> Result<u64, Error> {
    Ok(match config.uses_pointer_table.unwrap_or(false) {
        false => base_addr + index as u64 * config.element_size,
        true => mem
            .read_u64(base_addr + index as u64 * 8)
            .ok_or(format_err! {"Can't load pointer table index {}", index})?,
    })
}

pub fn read_enum<T: Sized + Default + FromPrimitive>(
    e: &mut T,
    mem: &dyn MemReader,
    addr: u64,
) -> Result<(), Error> {
    match size_of_val(e) {
        1 => {
            let v = mem
                .read_u8(addr)
                .ok_or(format_err!("Can't read at %0x{:x}", addr))?;
            *e = T::from_u8(v).unwrap_or(Default::default());
        }
        2 => {
            let v = mem
                .read_u16(addr)
                .ok_or(format_err!("Can't read at %0x{:x}", addr))?;
            *e = T::from_u16(v).unwrap_or(Default::default());
        }
        4 => {
            let v = mem
                .read_u32(addr)
                .ok_or(format_err!("Can't read at %0x{:x}", addr))?;
            *e = T::from_u32(v).unwrap_or(Default::default());
        }
        8 => {
            let v = mem
                .read_u64(addr)
                .ok_or(format_err!("Can't read at %0x{:x}", addr))?;
            *e = T::from_u64(v).unwrap_or(Default::default());
        }
        s => return Err(format_err!("Unsupported enums of size {}.", s)),
    };
    Ok(())
}
