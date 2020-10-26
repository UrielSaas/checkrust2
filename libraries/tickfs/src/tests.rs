use crate::error_codes::ErrorCode;
use crate::flash_controller::FlashController;
use crate::tickfs::{TickFS, HASH_OFFSET, LEN_OFFSET, VERSION, VERSION_OFFSET};
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;

fn check_region_main(buf: &[u8]) {
    // Check the version
    assert_eq!(buf[VERSION_OFFSET], VERSION);

    // Check the length
    assert_eq!(buf[LEN_OFFSET], 0x80);
    assert_eq!(buf[LEN_OFFSET + 1], 19);

    // Check the hash
    assert_eq!(buf[HASH_OFFSET + 0], 0x13);
    assert_eq!(buf[HASH_OFFSET + 1], 0x67);
    assert_eq!(buf[HASH_OFFSET + 2], 0xd3);
    assert_eq!(buf[HASH_OFFSET + 3], 0xe4);
    assert_eq!(buf[HASH_OFFSET + 4], 0xe0);
    assert_eq!(buf[HASH_OFFSET + 5], 0x9b);
    assert_eq!(buf[HASH_OFFSET + 6], 0xf7);
    assert_eq!(buf[HASH_OFFSET + 7], 0x6e);

    // Check the check hash
    assert_eq!(buf[HASH_OFFSET + 8], 0xdb);
    assert_eq!(buf[HASH_OFFSET + 9], 0x6d);
    assert_eq!(buf[HASH_OFFSET + 10], 0x81);
    assert_eq!(buf[HASH_OFFSET + 11], 0xc6);
    assert_eq!(buf[HASH_OFFSET + 12], 0x6b);
    assert_eq!(buf[HASH_OFFSET + 13], 0x95);
    assert_eq!(buf[HASH_OFFSET + 14], 0x50);
    assert_eq!(buf[HASH_OFFSET + 15], 0xdc);
}

fn check_region_one(buf: &[u8]) {
    // Check the version
    assert_eq!(buf[VERSION_OFFSET], VERSION);

    // Check the length
    assert_eq!(buf[LEN_OFFSET], 0x80);
    assert_eq!(buf[LEN_OFFSET + 1], 51);

    // Check the hash
    assert_eq!(buf[HASH_OFFSET + 0], 0x81);
    assert_eq!(buf[HASH_OFFSET + 1], 0x13);
    assert_eq!(buf[HASH_OFFSET + 2], 0x7e);
    assert_eq!(buf[HASH_OFFSET + 3], 0x95);
    assert_eq!(buf[HASH_OFFSET + 4], 0x9e);
    assert_eq!(buf[HASH_OFFSET + 5], 0x93);
    assert_eq!(buf[HASH_OFFSET + 6], 0xaa);
    assert_eq!(buf[HASH_OFFSET + 7], 0x3d);

    // Check the value
    assert_eq!(buf[HASH_OFFSET + 8], 0x23);
    assert_eq!(buf[28], 0x23);
    assert_eq!(buf[42], 0x23);

    // Check the check hash
    assert_eq!(buf[43], 0x08);
    assert_eq!(buf[44], 0x05);
    assert_eq!(buf[45], 0x89);
    assert_eq!(buf[46], 0xef);
    assert_eq!(buf[47], 0x5d);
    assert_eq!(buf[48], 0x42);
    assert_eq!(buf[49], 0x42);
    assert_eq!(buf[50], 0xdc);
}

fn check_region_two(buf: &[u8]) {
    // Check the version
    assert_eq!(buf[VERSION_OFFSET], VERSION);

    // Check the length
    assert_eq!(buf[LEN_OFFSET], 0x80);
    assert_eq!(buf[LEN_OFFSET + 1], 51);

    // Check the hash
    assert_eq!(buf[HASH_OFFSET + 0], 0x9d);
    assert_eq!(buf[HASH_OFFSET + 1], 0xd3);
    assert_eq!(buf[HASH_OFFSET + 2], 0x71);
    assert_eq!(buf[HASH_OFFSET + 3], 0x45);
    assert_eq!(buf[HASH_OFFSET + 4], 0x05);
    assert_eq!(buf[HASH_OFFSET + 5], 0xc2);
    assert_eq!(buf[HASH_OFFSET + 6], 0xf8);
    assert_eq!(buf[HASH_OFFSET + 7], 0x66);

    // Check the value
    assert_eq!(buf[HASH_OFFSET + 8], 0x23);
    assert_eq!(buf[28], 0x23);
    assert_eq!(buf[42], 0x23);

    // Check the check hash
    assert_eq!(buf[43], 0xdb);
    assert_eq!(buf[44], 0x1d);
    assert_eq!(buf[45], 0xd4);
    assert_eq!(buf[46], 0x8a);
    assert_eq!(buf[47], 0x7b);
    assert_eq!(buf[48], 0x39);
    assert_eq!(buf[49], 0x53);
    assert_eq!(buf[50], 0x8f);
}

/// Tests using a NOP flash controller
mod simple_flash_ctrl {
    use super::*;

    struct FlashCtrl {}

    impl FlashCtrl {
        fn new() -> Self {
            Self {}
        }
    }

    impl FlashController for FlashCtrl {
        fn read_region(
            &self,
            _region_number: usize,
            _offset: usize,
            buf: &mut [u8],
        ) -> Result<(), ErrorCode> {
            for b in buf.iter_mut() {
                *b = 0xFF;
            }

            Ok(())
        }

        fn write(&self, _address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
            check_region_main(buf);

            Ok(())
        }

        fn erase_region(&self, _region_number: usize) -> Result<(), ErrorCode> {
            Ok(())
        }
    }

    #[test]
    fn test_init() {
        let mut read_buf: [u8; 2048] = [0; 2048];
        TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x20000,
            0x800,
        )
        .unwrap();
    }
}

/// Tests using a simple flash controller that can only erase once
mod single_erase_flash_ctrl {
    use super::*;

    struct FlashCtrl {
        run: Cell<u8>,
    }

    impl FlashCtrl {
        fn new() -> Self {
            Self { run: Cell::new(0) }
        }
    }

    impl FlashController for FlashCtrl {
        fn read_region(
            &self,
            _region_number: usize,
            _offset: usize,
            buf: &mut [u8],
        ) -> Result<(), ErrorCode> {
            for b in buf.iter_mut() {
                *b = 0xFF;
            }

            Ok(())
        }

        fn write(&self, _address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
            check_region_main(buf);

            Ok(())
        }

        fn erase_region(&self, _region_number: usize) -> Result<(), ErrorCode> {
            // There are 64 regions, ensure this doesn't erase any a second time
            assert_ne!(self.run.get(), 64);
            self.run.set(self.run.get() + 1);

            Ok(())
        }
    }

    #[test]
    fn test_double_init() {
        let mut read_buf1: [u8; 2048] = [0; 2048];
        TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf1,
            0x20000,
            0x800,
        )
        .unwrap();

        let mut read_buf2: [u8; 2048] = [0; 2048];
        TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf2,
            0x20000,
            0x800,
        )
        .unwrap();
    }
}

/// Tests using a flash controller that can store data
mod store_flast_ctrl {
    use super::*;
    // An example FlashCtrl implementation
    struct FlashCtrl {
        buf: RefCell<[[u8; 1024]; 64]>,
        run: Cell<u8>,
    }

    impl FlashCtrl {
        fn new() -> Self {
            Self {
                buf: RefCell::new([[0xFF; 1024]; 64]),
                run: Cell::new(0),
            }
        }
    }

    impl FlashController for FlashCtrl {
        fn read_region(
            &self,
            region_number: usize,
            offset: usize,
            buf: &mut [u8],
        ) -> Result<(), ErrorCode> {
            println!("Read from region: {}", region_number);

            for (i, b) in buf.iter_mut().enumerate() {
                *b = self.buf.borrow()[region_number][offset + i]
            }

            Ok(())
        }

        fn write(&self, address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
            println!(
                "Write to address: {:#x}, region: {}",
                address,
                address / 1024
            );

            for (i, d) in buf.iter().enumerate() {
                self.buf.borrow_mut()[address / 1024][(address % 1024) + i] = *d;
            }

            // Check to see if we are adding a key
            if buf.len() > 1 {
                if self.run.get() == 0 {
                    println!("Writing main key: {:#x?}", buf);
                    check_region_main(buf);
                } else if self.run.get() == 1 {
                    println!("Writing key ONE: {:#x?}", buf);
                    check_region_one(buf);
                } else if self.run.get() == 2 {
                    println!("Writing key TWO: {:#x?}", buf);
                    check_region_two(buf);
                }
            }

            self.run.set(self.run.get() + 1);

            Ok(())
        }

        fn erase_region(&self, region_number: usize) -> Result<(), ErrorCode> {
            println!("Erase region: {}", region_number);
            let mut local_buf = self.buf.borrow_mut()[region_number];

            for d in local_buf.iter_mut() {
                *d = 0xFF;
            }

            Ok(())
        }
    }

    #[test]
    fn test_simple_append() {
        let mut read_buf: [u8; 1024] = [0; 1024];
        let tickfs = TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x10000,
            0x400,
        )
        .unwrap();

        let value: [u8; 32] = [0x23; 32];

        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();
        tickfs
            .append_key(&mut DefaultHasher::new(), b"TWO", &value)
            .unwrap();
    }

    #[test]
    fn test_double_append() {
        let mut read_buf: [u8; 1024] = [0; 1024];
        let tickfs = TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x10000,
            0x400,
        )
        .unwrap();

        let value: [u8; 32] = [0x23; 32];
        let mut buf: [u8; 32] = [0; 32];

        println!("Add key ONE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();

        println!("Get key ONE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"ONE", &mut buf)
            .unwrap();

        println!("Get non-existant key TWO");
        assert_eq!(
            tickfs.get_key(&mut DefaultHasher::new(), b"TWO", &mut buf),
            Err(ErrorCode::KeyNotFound)
        );

        println!("Add key ONE again");
        assert_eq!(
            tickfs.append_key(&mut DefaultHasher::new(), b"ONE", &value),
            Err(ErrorCode::KeyAlreadyExists)
        );

        println!("Add key TWO");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"TWO", &value)
            .unwrap();
        println!("Get key ONE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"ONE", &mut buf)
            .unwrap();
        println!("Get key TWO");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"TWO", &mut buf)
            .unwrap();

        println!("Get non-existant key THREE");
        assert_eq!(
            tickfs.get_key(&mut DefaultHasher::new(), b"THREE", &mut buf),
            Err(ErrorCode::KeyNotFound)
        );
    }

    #[test]
    fn test_append_and_delete() {
        let mut read_buf: [u8; 1024] = [0; 1024];
        let tickfs = TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x10000,
            0x400,
        )
        .unwrap();

        let value: [u8; 32] = [0x23; 32];
        let mut buf: [u8; 32] = [0; 32];

        println!("Add Key ONE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();

        println!("Get key ONE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"ONE", &mut buf)
            .unwrap();

        println!("Delete Key ONE");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"ONE")
            .unwrap();

        println!("Get non-existant key ONE");
        assert_eq!(
            tickfs.get_key(&mut DefaultHasher::new(), b"ONE", &mut buf),
            Err(ErrorCode::KeyNotFound)
        );

        println!("Try to delete Key ONE Again");
        assert_eq!(
            tickfs.invalidate_key(&mut DefaultHasher::new(), b"ONE"),
            Err(ErrorCode::KeyNotFound)
        );
    }

    #[test]
    fn test_garbage_collect() {
        let mut read_buf: [u8; 1024] = [0; 1024];
        let tickfs = TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x10000,
            0x400,
        )
        .unwrap();

        let value: [u8; 32] = [0x23; 32];
        let mut buf: [u8; 32] = [0; 32];

        println!("Garbage collect empty flash");
        assert_eq!(tickfs.garbage_collect(), Ok(0));

        println!("Add Key ONE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();

        println!("Garbage collect flash with valid key");
        assert_eq!(tickfs.garbage_collect(), Ok(0));

        println!("Delete Key ONE");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"ONE")
            .unwrap();

        println!("Garbage collect flash with deleted key");
        assert_eq!(tickfs.garbage_collect(), Ok(1024));

        println!("Get non-existant key ONE");
        assert_eq!(
            tickfs.get_key(&mut DefaultHasher::new(), b"ONE", &mut buf),
            Err(ErrorCode::KeyNotFound)
        );

        println!("Add Key ONE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();
    }
}

mod no_check_store_flast_ctrl {
    use super::*;
    // An example FlashCtrl implementation
    struct FlashCtrl {
        buf: RefCell<[[u8; 256]; 2]>,
    }

    impl FlashCtrl {
        fn new() -> Self {
            Self {
                buf: RefCell::new([[0xFF; 256]; 2]),
            }
        }
    }

    impl FlashController for FlashCtrl {
        fn read_region(
            &self,
            region_number: usize,
            offset: usize,
            buf: &mut [u8],
        ) -> Result<(), ErrorCode> {
            println!("Read from region: {}", region_number);

            for (i, b) in buf.iter_mut().enumerate() {
                *b = self.buf.borrow()[region_number][offset + i]
            }

            Ok(())
        }

        fn write(&self, address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
            println!(
                "Write to address: {:#x}, region: {}",
                address,
                address / 256
            );

            for (i, d) in buf.iter().enumerate() {
                self.buf.borrow_mut()[address / 256][(address % 256) + i] = *d;
            }

            Ok(())
        }

        fn erase_region(&self, region_number: usize) -> Result<(), ErrorCode> {
            println!("Erase region: {}", region_number);
            let mut local_buf = self.buf.borrow_mut()[region_number];

            for d in local_buf.iter_mut() {
                *d = 0xFF;
            }

            Ok(())
        }
    }
    #[test]
    fn test_region_full() {
        let mut read_buf: [u8; 256] = [0; 256];
        let tickfs = TickFS::<FlashCtrl, DefaultHasher>::new(
            FlashCtrl::new(),
            (&mut DefaultHasher::new(), &mut DefaultHasher::new()),
            &mut read_buf,
            0x200,
            0x100,
        )
        .unwrap();

        let value: [u8; 64] = [0x23; 64];
        let mut buf: [u8; 64] = [0; 64];

        println!("Add Key ONE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"ONE", &value)
            .unwrap();

        println!("Add Key TWO");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"TWO", &value)
            .unwrap();

        println!("Add Key THREE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"THREE", &value)
            .unwrap();

        println!("Add Key FOUR");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"FOUR", &value)
            .unwrap();

        println!("Add Key FIVE");
        tickfs
            .append_key(&mut DefaultHasher::new(), b"FIVE", &value)
            .unwrap();

        println!("Add Key SIX");
        assert_eq!(
            tickfs.append_key(&mut DefaultHasher::new(), b"SIX", &value),
            Err(ErrorCode::FlashFull)
        );

        println!("Get key ONE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"ONE", &mut buf)
            .unwrap();

        println!("Get key TWO");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"TWO", &mut buf)
            .unwrap();

        println!("Get key THREE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"THREE", &mut buf)
            .unwrap();

        println!("Get key FOUR");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"FOUR", &mut buf)
            .unwrap();

        println!("Get key FIVE");
        tickfs
            .get_key(&mut DefaultHasher::new(), b"FIVE", &mut buf)
            .unwrap();

        println!("Get key SIX");
        assert_eq!(
            tickfs.get_key(&mut DefaultHasher::new(), b"SIX", &mut buf),
            Err(ErrorCode::KeyNotFound)
        );

        println!("Delete Key ONE");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"ONE")
            .unwrap();

        println!("Delete Key TWO");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"TWO")
            .unwrap();

        println!("Delete Key THREE");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"THREE")
            .unwrap();

        println!("Delete Key FOUR");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"FOUR")
            .unwrap();

        println!("Delete Key FIVE");
        tickfs
            .invalidate_key(&mut DefaultHasher::new(), b"FIVE")
            .unwrap();

        println!("Delete Key SIX");
        assert_eq!(
            tickfs.invalidate_key(&mut DefaultHasher::new(), b"SIX"),
            Err(ErrorCode::KeyNotFound)
        );
    }
}
