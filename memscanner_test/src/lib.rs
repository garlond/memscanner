#[cfg(test)]
mod tests {
    use memscanner::test::TestMemReader;
    use memscanner::{Scannable, Signature, TypeConfig};

    use failure::{format_err, Error};

    #[derive(Debug, Default, Scannable)]
    struct TestObject {
        value1: u8,
        value2: u32,
    }

    #[derive(Debug, Default, Scannable)]
    struct StringTestObject {
        s: String,
    }

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

    fn get_string_limit_test_mem_reader() -> TestMemReader {
        #[rustfmt::skip]
        let r = TestMemReader {
            mem: vec![
                0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33,
                0x04, 0x00, 0x00, 0x00, 0x44, 0x55, 0x66, 0x77,
                0x4D, 0x65, 0x6D, 0x73, 0x63, 0x61, 0x6E, 0x6E,
                0x65, 0x72, 0x20, 0x69, 0x73, 0x20, 0x62, 0x65,
                0x73, 0x74, 0x20, 0x73, 0x63, 0x61, 0x6E, 0x6E,
                0x65, 0x72, 0x21, 0x20, 0x20, 0x4D, 0x65, 0x6D,
                0x73, 0x63, 0x61, 0x6E, 0x6E, 0x65, 0x72, 0x20,
                0x69, 0x73, 0x20, 0x62, 0x65, 0x73, 0x74, 0x20,
                0x73, 0x63, 0x61, 0x6E, 0x6E, 0x65, 0x72, 0x21,
            ],
            start_addr: 0x1000,
        };
        r
    }

    fn get_string_test_mem_reader() -> TestMemReader {
        #[rustfmt::skip]
        let r = TestMemReader {
            mem: vec![
                0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33,
                0x04, 0x00, 0x00, 0x00, 0x44, 0x55, 0x66, 0x77,
                0x4D, 0x65, 0x6D, 0x73, 0x63, 0x61, 0x6E, 0x6E,
                0x65, 0x72, 0x20, 0x69, 0x73, 0x20, 0x62, 0x65,
                0x73, 0x74, 0x20, 0x73, 0x63, 0x61, 0x6E, 0x6E,
                0x65, 0x72, 0x21, 0x00, 0x20, 0x4D, 0x65, 0x6D,
                0x73, 0x63, 0x61, 0x6E, 0x6E, 0x65, 0x72, 0x20,
                0x69, 0x73, 0x20, 0x62, 0x65, 0x73, 0x74, 0x20,
                0x73, 0x63, 0x61, 0x6E, 0x6E, 0x65, 0x72, 0x21,
            ],
            start_addr: 0x1000,
        };
        r
    }

    fn get_array_test_mem_reader() -> TestMemReader {
        #[rustfmt::skip]
        let r = TestMemReader {
            mem: vec![
                0xff, 0xff, 0xff, 0xff, 0x00, 0x11, 0x22, 0x33, // 0x1000
                0x04, 0x00, 0x00, 0x00, 0x44, 0x55, 0x66, 0x77, // 0x1008
                0x28, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 0x1010
                0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 0x1018
                0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, // 0x1020
                0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, // 0x1028
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

    fn get_string_test_type_config() -> TypeConfig {
        let mut text = "
        {
            signature: [\"asm(00112233^^^^^^^^********)\"],
            fields: {
                s: 0x0,
            }
        }"
        .as_bytes();
        TypeConfig::new(&mut text).unwrap()
    }

    fn get_array_test_type_config() -> TypeConfig {
        let mut text = "
        {
            signature: [\"asm(00112233^^^^^^^^********)\"],
            array: {
                element_size: 8,
                element_count: 2,
                uses_pointer_table: true,
            },
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
    fn object_test() -> Result<(), Error> {
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

    #[test]
    fn string_test() -> Result<(), Error> {
        let config = get_string_test_type_config();
        let mem = get_string_test_mem_reader();

        let resolver = StringTestObject::get_resolver(config)?;
        let scanner = resolver(&mem, mem.start_addr, mem.start_addr + mem.mem.len() as u64)?;

        let mut obj: StringTestObject = Default::default();
        scanner(&mut obj, &mem)?;
        assert_eq!(obj.s, "Memscanner is best scanner!");

        Ok(())
    }

    #[test]
    fn string_limit_test() -> Result<(), Error> {
        let config = get_string_test_type_config();
        let mem = get_string_limit_test_mem_reader();

        let resolver = StringTestObject::get_resolver(config)?;
        let scanner = resolver(&mem, mem.start_addr, mem.start_addr + mem.mem.len() as u64)?;

        let mut obj: StringTestObject = Default::default();
        scanner(&mut obj, &mem)?;
        assert_eq!(obj.s.len(), 32);
        assert_eq!(obj.s, "Memscanner is best scanner!  Mem");

        Ok(())
    }

    #[test]
    fn array_test() -> Result<(), Error> {
        let config = get_array_test_type_config();
        let mem = get_array_test_mem_reader();

        println!("{:?}", &config);
        let resolver = TestObject::get_array_resolver(config)?;
        let scanner = resolver(&mem, mem.start_addr, mem.start_addr + mem.mem.len() as u64)?;

        let mut obj = Vec::new();
        scanner(&mut obj, &mem)?;
        println!("{:?}", &obj);
        assert_eq!(obj[0].value1, 0x88);
        assert_eq!(obj[0].value2, 0xffeeddcc);
        assert_eq!(obj[1].value1, 0x00);
        assert_eq!(obj[1].value2, 0x77665544);

        Ok(())
    }
}
