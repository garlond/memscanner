use super::test::TestMemReader;
use super::ArrayConfig;
use super::MemReader;
use failure::{format_err, Error};

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
