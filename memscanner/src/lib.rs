pub mod process;
pub mod signature;
pub mod test;

use failure::Error;
use json5;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::Read;

pub use memscanner_derive::Scannable;
pub use signature::Signature;

macro_rules! read_type_impl {
    ($type: ty, $func_name: tt) => {
        fn $func_name (&self, addr: u64) -> Option<$type> {
            let len = std::mem::size_of::<$type>();
            let mut buf: Vec<u8> = vec![0; len];
            let read_bytes = self.read(&mut buf, addr, len);
            if read_bytes != len {
                return None;
            }
            Some(<$type>::from_ne_bytes((&buf as &[u8]).try_into().ok()?))
        }
    }
}

macro_rules! read_float_impl {
    ($type: ty, $int_type: ty, $func_name: tt) => {
        fn $func_name (&self, addr: u64) -> Option<$type> {
            let len = std::mem::size_of::<$type>();
            let mut buf: Vec<u8> = vec![0; len];
            let read_bytes = self.read(&mut buf, addr, len);
            if read_bytes != len {
                return None;
            }
            Some(<$type>::from_bits(<$int_type>::from_ne_bytes((&buf as &[u8]).try_into().ok()?)))
        }
    }
}

/// The `MemReader` trait allows for reading bytes form a memory source.
pub trait MemReader {
    /// Read bytes `len` bytes at `addr` from the `MemReader` and write them
    /// to `buf`.  
    ///
    /// Returns: number of bytes actually read.
    fn read(&self, buf: &mut [u8], addr: u64, len: usize) -> usize;

    fn read_u8(&self, addr: u64) -> Option<u8> {
        let mut val: Vec<u8> = vec![0; 1];
        let read_bytes = self.read(&mut val, addr, 1);
        if read_bytes != 1 {
            return None;
        }
        Some(val[0])
    }

    read_type_impl!(u16, read_u16);
    read_type_impl!(i16, read_i16);
    read_type_impl!(u32, read_u32);
    read_type_impl!(i32, read_i32);
    read_type_impl!(u64, read_u64);
    read_type_impl!(i64, read_i64);

    read_float_impl!(f32, u32, read_f32);
    read_float_impl!(f64, u64, read_f64);
}

#[derive(Debug, Deserialize)]
struct TypeConfigIntermediate {
    signature: Vec<String>,
    fields: HashMap<String, u64>,
}

/// A configuration describing how to find a piece of memory and map it to
/// a struct.
#[derive(Debug)]
pub struct TypeConfig {
    // Implement a custom deserializer for that type which parses the strings.
    // this will avoid the need for the intermediate type above.
    pub signature: signature::Signature,
    pub fields: HashMap<String, u64>,
}

impl TypeConfig {
    /// Read a json5 config.
    pub fn new(reader: &mut impl Read) -> Result<TypeConfig, Error> {
        let mut buffer = String::new();
        reader.read_to_string(&mut buffer)?;

        let inter: TypeConfigIntermediate = json5::from_str(&buffer)?;
        let sig = signature::Signature::new(&inter.signature)?;

        Ok(TypeConfig {
            signature: sig,
            fields: inter.fields,
        })
    }
}

/// A function capable of resolving the location of a `Scannable`.
///
/// # Arguments
/// * `mem_reader` - The `MemReader` to use to resolve the `Scannable`.
/// * `start_addr` - Address to start the resolution at.
/// * `end_addr` - Address to end the resolution at. (non-inclusive)
///
/// Returns a `Scanner` for reading the `Scannable`
pub type Resolver<T> = dyn Fn(&dyn MemReader, u64, u64) -> Result<Box<Scanner<T>>, Error>;

/// A function capable of reading a `Scannable` from a `MemReader`
///
/// # Arguments
/// * `obj` - Scannable to update.
/// * `mem_reader` - The `MemReader` to use to resolve the `Scannable`.
pub type Scanner<T> = dyn Fn(&mut T, &dyn MemReader) -> Result<(), Error>;

/// A struct that can be scanned with `memscanner`.
///
/// This is normally implemented through the `#[derive(Scannable)]` macro.
pub trait Scannable {
    /// Returns a `Resolver` capable of finding the `Scannable` described by
    /// the `config`.
    fn get_resolver(config: TypeConfig) -> Result<Box<Resolver<Self>>, Error>;
}