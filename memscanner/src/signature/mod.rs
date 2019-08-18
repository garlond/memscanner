mod parser;

use super::MemReader;
use failure::{format_err, Error};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Match {
    Any,
    Position,
    Literal(u8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Op {
    Asm(Vec<Match>),
    Ptr(i32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    ops: Vec<Op>,
}

impl Signature {
    pub fn new(ops: &Vec<String>) -> Result<Signature, Error> {
        let mut sig = Signature { ops: vec![] };

        for op_str in ops {
            let (_, op) =
                parser::parse_op(op_str).map_err(|_| format_err!("Can't parse op: {}", op_str))?;
            sig.ops.push(op);
        }

        Ok(sig)
    }

    pub fn resolve(&self, mem: &dyn MemReader, start_addr: u64, end_addr: u64) -> Option<u64> {
        let mut addr = start_addr;
        for op in &self.ops {
            addr = match &op {
                Op::Asm(p) => resolve_asm(mem, start_addr, end_addr, &p)?,
                Op::Ptr(o) => resolve_ptr(mem, addr, *o)?,
            };
        }
        Some(addr)
    }
}

// Offset a u64 address by and i32 offset.  Takes care to not lose the upper
// bit of the u64.  Does not handle over/underflow.
fn offset_addr(addr: u64, offset: i32) -> u64 {
    if offset < 0 {
        addr - (-offset) as u64
    } else {
        addr + offset as u64
    }
}

// Check if `pattern` matches the contents of `mem` at `start_addr`
fn check_pattern(
    mem: &dyn MemReader,
    start_addr: u64,
    end_addr: u64,
    pattern: &[Match],
) -> Option<u64> {
    if end_addr - start_addr < pattern.len() as u64 {
        // TODO: plumb Results through this whole thing.
        println!("Not enough room for pattern");
        return None;
    }

    // Read all the necessary bytes for the patter.
    let mut mem_contents = vec![0x0; pattern.len()];
    let mem_read = mem.read(&mut mem_contents, start_addr, pattern.len());
    if mem_read != pattern.len() {
        println!("incomplete read");
        return None;
    }

    // Determine if mem_contents matches the pattern.
    let mut offset: Option<u64> = None;
    for i in 0..pattern.len() {
        match &pattern[i] {
            Match::Position => {
                // Store the offset of the first match token.
                if offset == None {
                    offset = Some(i as u64);
                }
            }
            Match::Any => {}
            Match::Literal(val) => {
                if mem_contents[i] != *val {
                    return None;
                }
            }
        };
    }

    // If there were no position tokens, return the end of the match.
    match offset {
        None => Some(pattern.len() as u64),
        Some(_) => offset,
    }
}

// Scan through `mem` from `start_addr` to `end_addr` looking for a
// pattern match.
fn resolve_match(
    mem: &dyn MemReader,
    start_addr: u64,
    end_addr: u64,
    pattern: &[Match],
) -> Option<u64> {
    let mem_len = end_addr - start_addr;
    for i in 0..=(mem_len as usize - pattern.len()) {
        if let Some(offset) = check_pattern(mem, start_addr + i as u64, end_addr, pattern) {
            return Some(start_addr + offset + i as u64);
        }
    }

    None
}

// Scan through `mem` from `start_addr` to `end_addr` looking for a
// pattern match.  Then treat it as an argument to an indirect load holding
// a pointer. Look up that location an return its contents.
fn resolve_asm(
    mem: &dyn MemReader,
    start_addr: u64,
    end_addr: u64,
    pattern: &[Match],
) -> Option<u64> {
    let match_addr = resolve_match(mem, start_addr, end_addr, pattern)?;
    let offset = mem.read_i32(match_addr)?;
    let addr = offset_addr(match_addr, offset) + 4;
    Some(addr)
}

// Look up the contents of `addr` (offset by `offset`) and return its contents.
fn resolve_ptr(mem: &dyn MemReader, addr: u64, offset: i32) -> Option<u64> {
    let addr = (addr as i64 + offset as i64) as u64;
    let addr = mem.read_u64(addr)?;
    Some(addr)
}

#[cfg(test)]
mod tests {
    use super::super::test::TestMemReader;
    use super::*;

    #[test]
    fn single_lea() {
        #[rustfmt::skip]
        let mem = TestMemReader {
            mem: vec![
                0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33,
                0x04, 0x00, 0x00, 0x00, 0x44, 0x55, 0x66, 0x77,
                0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
            ],
            start_addr: 0x1000,
        };

        let sig = Signature::new(&vec!["asm(00112233^^^^^^^^********)".to_string()]).unwrap();
        println!("{:?}", sig);
        let offset = sig
            .resolve(&mem, mem.start_addr, mem.start_addr + mem.mem.len() as u64)
            .unwrap();
        assert_eq!(offset, 0x1010);
        assert_eq!(mem.read_u64(offset).unwrap(), 0xffeeddccbbaa9988);
    }
}
