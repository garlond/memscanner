pub mod macro_helpers;
pub mod process;
pub mod signature;
pub mod test;

use failure::Error;
use json5;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::Read;

pub use memscanner_derive::{Scannable, ScannableEnum};
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

    fn read_string(&self, addr: u64) -> Option<String> {
        let string_limit = 32;
        let mut bytes: Vec<u8> = Vec::new();

        for i in 0..string_limit {
            let b = self.read_u8(addr + i)?;
            if b == 0x0 {
                break;
            }
            bytes.push(b);
        }

        Some(String::from_utf8_lossy(&bytes).to_string())
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

#[derive(Clone, Debug, Deserialize)]
pub struct ArrayConfig {
    pub element_size: u64,
    pub element_count: u64,
    pub uses_pointer_table: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TypeConfigIntermediate {
    signature: Vec<String>,
    array: Option<ArrayConfig>,
    fields: HashMap<String, u64>,
}

/// A configuration describing how to find a piece of memory and map it to
/// a struct.
#[derive(Clone, Debug)]
pub struct TypeConfig {
    // TODO: Implement a custom deserializer for that type which parses the strings.
    // this will avoid the need for the intermediate type above.
    pub signature: signature::Signature,
    pub array: Option<ArrayConfig>,
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
            array: inter.array,
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

pub type ArrayResolver<T> = dyn Fn(&dyn MemReader, u64, u64) -> Result<Box<ArrayScanner<T>>, Error>;
/// A function capable of reading a `Scannable` from a `MemReader`
///
/// # Arguments
/// * `obj` - Scannable to update.
/// * `mem_reader` - The `MemReader` to use to resolve the `Scannable`.
pub type Scanner<T> = dyn Fn(&mut T, &dyn MemReader) -> Result<(), Error>;

pub type ArrayScanner<T> = dyn Fn(&mut Vec<T>, &dyn MemReader) -> Result<(), Error>;
/// A struct that can be scanned with `memscanner`.
///
/// This is normally implemented through the `#[derive(Scannable)]` macro.
pub trait Scannable
where
    Self: std::marker::Sized,
{
    /// Returns a `Resolver` capable of finding the `Scannable` described by
    /// the `config`.
    fn get_resolver(config: TypeConfig) -> Result<Box<Resolver<Self>>, Error>;
    /// Returns a `Resolver` capable of finding the `Scannable` described by
    /// the `config`.  This `Scannable` will read into a Vec.
    fn get_array_resolver(config: TypeConfig) -> Result<Box<ArrayResolver<Self>>, Error>;
}

/// A value that can be scanned as a member of a `Scannable` struct.
pub trait ScannableValue<T> {
    /// Scans the value at `addr` using `mem` to read its value.
    fn scan_val(&mut self, mem: &dyn MemReader, addr: u64) -> Result<(), Error>;
}

// A macro to generate implementations of ScannableValue for types that have
// direct MemReader readers.
macro_rules! scannable_value_impl {
    ($type: ty, $func_name: tt) => {
        impl ScannableValue<$type> for $type {
            fn scan_val(&mut self, mem: &dyn MemReader, addr: u64) -> Result<(), Error> {
                use failure::format_err;
                *self = mem
                    .$func_name(addr)
                    .ok_or(format_err!("can't read value"))?;
                Ok(())
            }
        }
    };
}

scannable_value_impl!(String, read_string);

scannable_value_impl!(u8, read_u8);
scannable_value_impl!(u16, read_u16);
scannable_value_impl!(i16, read_i16);
scannable_value_impl!(u32, read_u32);
scannable_value_impl!(i32, read_i32);
scannable_value_impl!(u64, read_u64);
scannable_value_impl!(i64, read_i64);

scannable_value_impl!(f32, read_f32);
scannable_value_impl!(f64, read_f64);
