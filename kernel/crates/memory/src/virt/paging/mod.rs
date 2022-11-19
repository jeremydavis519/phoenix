/* Copyright (c) 2018-2021 Jeremy Davis (jeremydavis519@gmail.com)
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software
 * and associated documentation files (the "Software"), to deal in the Software without restriction,
 * including without limitation the rights to use, copy, modify, merge, publish, distribute,
 * sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or
 * substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
 * NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
 * NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
 * DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

//! This module defines the paging structure for the architecture for which the kernel is being
//! built.

#[cfg(target_arch = "aarch64")] pub mod aarch64;
#[cfg(target_arch = "aarch64")] pub use self::aarch64::*;

#[cfg(target_arch = "x86_64")] pub mod temp {
    #![doc(hidden)]
    // TODO: These are here just to allow the unit tests to compile.
    use { alloc::alloc::AllocError, core::num::NonZeroUsize, crate::phys::{RegionType, block::BlockMut} };
    #[allow(missing_docs)]
    pub fn page_size() -> usize { unimplemented!(); }
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub struct RootPageTable;
    #[allow(missing_docs)]
    impl RootPageTable {
        pub fn new_userspace(_asid: u16) -> Result<BlockMut<RootPageTable>, AllocError> { unimplemented!(); }
        pub fn map(
                &self,
                _phys_base: usize,
                _virt_base: Option<usize>,
                _size: NonZeroUsize,
                _reg_type: RegionType
        ) -> Result<usize, ()> {
            unimplemented!()
        }
        pub fn map_zeroed(&self, _virt_base: usize, _size: NonZeroUsize) -> Result<(), ()> {
            unimplemented!()
        }
        pub fn map_exe_file(&self, _virt_base: Option<usize>, _size: NonZeroUsize) -> Result<usize, ()> {
            unimplemented!()
        }
        pub fn map_from_exe_file(
                &self,
                _phys_base: usize,
                _virt_base: usize,
                _size: NonZeroUsize,
                _reg_type: RegionType
        ) -> Result<(), ()> {
            unimplemented!()
        }
        pub fn map_zeroed_from_exe_file(&self, _virt_base: usize, _size: NonZeroUsize) -> Result<(), ()> {
            unimplemented!()
        }
        pub fn userspace_addr_to_kernel_addr<E: FnOnce(usize, &mut [u8]) -> Result<(), ()>>(
            &self,
            _userspace_addr: usize,
            _region_type: RegionType,
            _read_exe_file: E,
        ) -> Option<usize> {
            unimplemented!()
        }
    }
    #[allow(missing_docs)]
    pub fn trampoline<T>(ptr: *const T) -> *const T { ptr }
}
#[cfg(target_arch = "x86_64")] pub use self::temp::*;
