#[cfg(test)]
mod tests {
    use memscanner::test::TestMemReader;
    use memscanner::{Scannable, Signature, TypeConfig};

    use failure::{format_err, Error};

    #[derive(Default, Scannable)]
    struct TestObject {
        value1: u8,
        value2: u32,
    }

    // This that should be emitted by the derive macro
    fn get_test_mem_reader() -> TestMemReader {
        #[rustfmt::skip]
        let r = TestMemReader {
            mem: vec![
                0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33,
                0x04, 0x00, 0x00, 0x00, 0x44, 0x55, 0x66, 0x77,
                0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
            ],
            start_addr: 0x1000,
        };
        r
    }

    fn get_test_type_config() -> TypeConfig {
        let mut text = "
        {
            signature: [\"asm(00112233^^^^^^^^********)\"],
            fields: {
                value1: 0x0,
                value2: 0x4,
            }
        }"
        .as_bytes();
        TypeConfig::new(&mut text).unwrap()
    }

    #[test]
    fn type_config_test() {
        let config = get_test_type_config();
        assert_eq!(
            config.signature,
            Signature::new(&vec!["asm(00112233^^^^^^^^********)".to_string()]).unwrap()
        );
        assert_eq!(
            config.fields,
            [("value1".to_string(), 0u64), ("value2".to_string(), 4u64)]
                .iter()
                .cloned()
                .collect()
        );
    }

    #[test]
    fn prototype_test() -> Result<(), Error> {
        let config = get_test_type_config();
        let mem = get_test_mem_reader();

        let resolver = TestObject::get_resolver(config)?;
        let scanner = resolver(&mem, mem.start_addr, mem.start_addr + mem.mem.len() as u64)?;

        let mut obj: TestObject = Default::default();
        scanner(&mut obj, &mem)?;
        assert_eq!(obj.value1, 0x88);
        assert_eq!(obj.value2, 0xffeeddcc);

        Ok(())
    }
}
