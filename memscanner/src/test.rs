use super::MemReader;

/// A `MemReader` implementation that is backed by a buffer.  Useful for
/// writing tests.
pub struct TestMemReader {
    pub mem: Vec<u8>,
    pub start_addr: u64,
}

impl MemReader for TestMemReader {
    fn read(&self, buf: &mut [u8], addr: u64, len: usize) -> usize {
        let index = (addr - self.start_addr) as usize;
        let read_len = if index + len > self.mem.len() {
            self.mem.len() - index
        } else {
            len
        };

        buf.copy_from_slice(&self.mem[index..(index + read_len)]);

        read_len
    }
}
