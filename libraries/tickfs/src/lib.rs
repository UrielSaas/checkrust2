//! TickFS (Tiny Circular Key Value File System) is a small file system allowing
//! key value pairs to be stored in Flash Memory.
//!
//! TickFS was written to allow the Tock OS kernel to persistently store app data
//! on flash. It was written to be generic though, so other Rust applications can
//! use it if they want.
//!
//! TickFS is based on similar concepts as
//! [Yaffs1](https://yaffs.net/documents/how-yaffs-works]).
//!
//! # Goals of TickFS
//!
//! TickFS is designed with these main goals (in order)
//!
//!  * Fully implemented in no_std Rust
//!  * Power loss resilient
//!  * Maintain data integrity and detect media errors
//!  * Wear leveling
//!  * Low memory usage
//!  * Low storage overhead
//!  * No external crates in use (not including unit tests)
//!
//! TickFS is also designed with some assumptions
//!
//!  * Most operations will be retrieving keys
//!  * Some operations will be storing keys
//!  * Keys will rarely be deleted
//!  * Key values will rarely need to be modified
//!
//! # How TickFS works
//!
//! Unlike a regular File System (FS) TickFS is only designed to store Key/Value (KV)
//! pairs in flash. It does not support writing actual files, directories or other
//! complex objects. Although a traditional file system layer could be added on top
//! to add such features.
//!
//! TickFS allows writing new key/value pairs (by appending them) and removing
//! old key/value pairs.
//!
//! TickFS has two important types, regions and objects.
//!
//! A TickFS region is the smallest region of the flash memory that can be erased
//! in a single command.
//!
//! TickFS saves and restores objects from flash. TickFS objects contain the value
//! the user wanted to store as well as extra header data. Objects are internal to
//! TickFS and users don't need to understand them in detail to use it.
//!
//! To see the full TickFS spec check the [README.md file](https://github.com/tock/tock/blob/master/libraries/tickfs/README.md).
//!
//! TickFS provides ACID properties.
//!
//! # Using TickFS
//!
//! To use TickFS first you need to implemented the `FlashCtrl` trait. The
//! example below is for 1024 byte region sizes.
//!
//! Then you will need to create a TickFS implementation.
//!
//!
//! ```rust
//! // EXAMPLE ONLY: The `DefaultHasher` is subject to change
//! // and hence is not a good fit.
//! use std::collections::hash_map::DefaultHasher;
//! use std::cell::RefCell;
//! use tickfs::TickFS;
//! use tickfs::error_codes::ErrorCode;
//! use tickfs::flash_controller::FlashController;
//!
//! struct FlashCtrl {
//!     buf: RefCell<[[u8; 1024]; 64]>,
//! }
//!
//! impl FlashCtrl {
//!     fn new() -> Self {
//!         Self {
//!             buf: RefCell::new([[0xFF; 1024]; 64]),
//!         }
//!     }
//! }
//!
//! impl FlashController<1024> for FlashCtrl {
//!     fn read_region(&self, region_number: usize, offset: usize, buf: &mut [u8; 1024]) -> Result<(), ErrorCode> {
//!         // TODO: Read the specified flash region
//!         for (i, b) in buf.iter_mut().enumerate() {
//!             *b = self.buf.borrow()[region_number][offset + i]
//!         }
//!         Ok(())
//!     }
//!
//!     fn write(&self, address: usize, buf: &[u8]) -> Result<(), ErrorCode> {
//!         // TODO: Write the data to the specified flash address
//!         for (i, d) in buf.iter().enumerate() {
//!             self.buf.borrow_mut()[address / 1024][(address % 1024) + i] = *d;
//!         }
//!         Ok(())
//!     }
//!
//!     fn erase_region(&self, region_number: usize) -> Result<(), ErrorCode> {
//!         // TODO: Erase the specified flash region
//!         Ok(())
//!     }
//! }
//!
//! let mut read_buf: [u8; 1024] = [0; 1024];
//! let tickfs = TickFS::<FlashCtrl, DefaultHasher, 1024>::new(FlashCtrl::new(),
//!                   &mut read_buf, 0x1000);
//! tickfs
//!    .initalise((&mut DefaultHasher::new(), &mut DefaultHasher::new()))
//!    .unwrap();
//!
//! // Add a key
//! let value: [u8; 32] = [0x23; 32];
//! tickfs.append_key(&mut DefaultHasher::new(), b"ONE", &value).unwrap();
//!
//! // Get the same key back
//! let mut buf: [u8; 32] = [0; 32];
//! tickfs.get_key(&mut DefaultHasher::new(), b"ONE", &mut buf).unwrap();
//! ```
//!
//! You can then use the `get_key()` function to get the key back from flash.
//!
//! # Collisions
//!
//! TickFS will prevent a new key/value pair with a colliding hash of the key to be
//! added. The collision will be reported to the user with the `KeyAlreadyExists`
//! `ErroCode`.
//!
//! # Power loss protection
//!
//! TickFS ensures that in the event of a power loss, no stored data is lost or
//! corrupted. The only data that can be lost in the event of a power loss is
//! the data currently being written (if it hasn't been write to flash yet).
//!
//! If a power loss occurs after calling `append_key()` or `invalidate_key()`
//! before it has completed then the operation probably did not complete and
//! that data is lost.
//!
//! To help reduce this time to be as short as possible the `FlashController`
//! is synchronous. Although flash writes can take a considerable amount of time
//! and this will stall the application, this still seems like a good idea
//! to avoid loosing data.
//!
//! # Security
//!
//! TickFS uses check sums to check data integrity. TickFS does not have any measures
//! to prevent malicious manipulation or privacy. An attacker with access to the
//! flash can change the values without being detected. An attacked with access
//! to flash can also read all of the information. Any privacy, security or
//! authentication measures need to be layered on top of TickFS.
//!
//! # Hash Function
//!
//! Any hash function that implements Rust's `core::hash::Hasher` trait can be used.
//!
//! The hash function ideally should generate uniform hashes and must not change during
//! the lifetime of the filesystem.
//!
//! The Rust `core::hash::Hasher` implementation is a little strange. When the
//! hash is calculated with the `finish()` function the internal state of the
//! `Hasher` is not reset. This means that the check sum is generated with the
//! following code and the key input becomes part of the check sum.
//!
//! ```rust,ignore
//!         key.hash(hash_function);
//!         let hash = hash_function.finish();
//!
//!         buf.hash(hash_function);
//!         value.hash(hash_function);
//!         let check_sum = hash_function.finish();
//! ```

#![feature(min_const_generics)]
#![no_std]
#![deny(missing_docs)]

pub mod async_ops;
pub mod error_codes;
pub mod flash_controller;
pub mod success_codes;
pub mod tickfs;

// Use this to generate nicer docs
#[doc(inline)]
pub use crate::async_ops::AsyncTickFS;
#[doc(inline)]
pub use crate::error_codes::ErrorCode;
#[doc(inline)]
pub use crate::flash_controller::FlashController;
#[doc(inline)]
pub use crate::tickfs::TickFS;

// This is used to run the tests on a host
#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests;
