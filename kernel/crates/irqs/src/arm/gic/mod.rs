/* Copyright (c) 2017-2021 Jeremy Davis (jeremydavis519@gmail.com)
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

pub mod irq;

use {
    core::{
        arch::asm,
        sync::atomic::{AtomicU8, Ordering}
    },
    volatile::Volatile,
    bitflags::bitflags,

    i18n::Text,
    io::printlndebug,
    memory::{
        allocator::AllMemAlloc,
        phys::block::Mmio
    },
    shared::lazy_static,

    self::irq::IrqTrigger
};

/// Handles reading and writing registers in the GIC distributor.
struct DistRegs {
    block: Mmio<Volatile<u8>>
}

/// Handles reading and writing registers in the GIC CPU interface.
struct CpuRegs {
    block: Mmio<Volatile<u8>>,
    arch_version: u32 // The version of the GIC spec to which this interface conforms
}

impl DistRegs {
    fn new(base: usize, size: usize) -> Self {
        if let Ok(block) = AllMemAlloc.mmio_mut(base, size) {
            DistRegs { block }
        } else {
            panic!("{}", Text::GicCouldntReserveDistBlock);
        }
    }

    /// Reads the value of the given register.
    pub fn read(&self, reg: DistMmio) -> u32 {
        if !Self::is_reg_readable(reg as usize) {
            panic!("{}", Text::GicReadUnreadableDistReg(reg as usize));
        }

        unsafe { (&*(self.block.index(reg as usize) as *const _ as *const Volatile<u32>)).read() }
    }

    /// Writes the given value to the given register.
    pub fn write(&self, reg: DistMmio, val: u32) {
        if !Self::is_reg_writable(reg as usize) {
            panic!("{}", Text::GicWriteUnwritableDistReg(reg as usize));
        }

        unsafe { (&mut *(self.block.index(reg as usize) as *mut _ as *mut Volatile<u32>)).write(val); }
    }

    /*/// Reads a single byte from the given register.
    /// Warning: Most registers are not guaranteed to work with single-byte accesses.
    pub fn read_byte(&self, reg: DistMmio, byte_offset: usize) -> u8 {
        let reg = reg as usize + byte_offset;
        if !Self::is_reg_readable(reg / 4 * 4) {
            panic!("{}", Text::GicReadUnreadableDistReg(reg));
        }

        unsafe { self.block.index(reg).read() }
    }*/

    /// Writes a single byte to the given register.
    /// Warning: Most registers are not guaranteed to work with single-byte accesses.
    pub fn write_byte(&self, reg: DistMmio, byte_offset: usize, val: u8) {
        let reg = reg as usize + byte_offset;
        if !Self::is_reg_writable(reg / 4 * 4) {
            panic!("{}", Text::GicWriteUnwritableDistReg(reg));
        }

        unsafe { (*self.block.index(reg)).write(val); }
    }

    pub fn enable_irq(&self, irq: usize, trigger: IrqTrigger) {
        assert!(irq < 1020);

        static CPU_COUNTER: AtomicU8 = AtomicU8::new(0);

        // Set the interrupt to either edge- or level-triggered.
        unsafe {
            let reg_index = irq / 16;
            let bit_index = (irq % 16) * 2;
            let reg = &mut *(self.block.index(DistMmio::ICFGR0 as usize + reg_index * 4) as *mut _ as *mut Volatile<u32>);
            let mut icfgr = reg.read() & !(3 << bit_index);
            match trigger {
                IrqTrigger::Level => {},
                IrqTrigger::Edge => {
                    icfgr |= 2 << bit_index;
                }
            }
            reg.write(icfgr);
        }

        let reg_index = irq / 32;
        let bit_index = irq % 32;

        if reg_index == 0 {
            // TODO: GICD_ISENABLER0 is banked and has a separate instance for each CPU (at least
            // the CPUs with GICR_TYPER.ProcessorNumber < 8; all others may or may not have their
            // own banked copies). Make sure this CPU's ProcessorNumber is less than 8, and if it
            // isn't, send an IPI to one of the first 8 CPUs to have it enable the interrupt
            // instead.
            //if `ProcessorNumber` >= 8 {
                let _target_cpu = CPU_COUNTER.fetch_add(1, Ordering::AcqRel) % cpu_count_without_affinity_routing();
                //...
            //}
        }

        unsafe { (&mut *(self.block.index(0x0100 + reg_index * 4) as *mut _ as *mut Volatile<u32>)).write(1 << bit_index); }
    }

    pub fn disable_irq(&self, irq: usize) {
        assert!(irq < 1020);

        let reg_index = irq / 32;
        let bit_index = irq % 32;

        if reg_index == 0 {
            // TODO: GICD_ICENABLER0 is banked and has a separate instance for each CPU (at least
            // the CPUs with GICR_TYPER.ProcessorNumber < 8; all others may or may not have their
            // own banked copies). Make sure the correct CPU disables the interrupt.
        }

        unsafe { (&mut *(self.block.index(0x0180 + reg_index * 4) as *mut _ as *mut Volatile<u32>)).write(1 << bit_index); }
    }

    /*pub fn route_irq(&self, _irq: usize, _affinity: u64) {
        // TODO: Use affinity routing (new in GICv3).
        unimplemented!();
        /*assert!(irq >= 32 && irq < 1020);

        unsafe {
            #[cfg(target_arch = "aarch64")] {
                (&mut *(self.block.index(0x6000 + irq * 8) as *mut _ as *mut Volatile<u64>)).write(affinity);
            } #[cfg(not(target_arch = "aarch64"))] {
                (&mut *(self.block.index(0x6000 + irq * 8) as *mut _ as *mut Volatile<u32>)).write(affinity as u32);
                (&mut *(self.block.index(0x6000 + irq * 8 + 4) as *mut _ as *mut Volatile<u32>)).write((affinity >> 32) as u32);
            }
        }*/
    }*/

    fn is_reg_readable(reg: usize) -> bool {
        use self::DistMmio::*;
        if reg as usize % 4 != 0 {
            return false;
        }
        match reg {
            x if x == CTLR as usize ||
                x == TYPER as usize ||
                x == IIDR as usize ||
                x == STATUSR as usize ||
                x == PIDR2 as usize => true,
            x => {
                (x >= IGROUPR0 as usize && x <= IPRIORITYR254 as usize) || (x >= ITARGETSR0 as usize && x <= IGRPMODR31 as usize) ||
                    (x >= NSACR0 as usize && x <= NSACR63 as usize) || (x >= CPENDSGIR0 as usize && x <= SPENDSGIR3 as usize) ||
                    (x >= IROUTER32 as usize && x <= IROUTER1019 as usize)
            }
        }
    }

    fn is_reg_writable(reg: usize) -> bool {
        use self::DistMmio::*;

        if reg as usize % 4 != 0 {
            return false;
        }
        match reg {
            x if x == CTLR as usize ||
                x == STATUSR as usize ||
                x == SETSPI_NSR as usize ||
                x == CLRSPI_NSR as usize ||
                x == SETSPI_SR as usize ||
                x == CLRSPI_SR as usize => true,
            x => {
                (x >= IGROUPR0 as usize && x <= IPRIORITYR254 as usize) || (x >= ITARGETSR0 as usize && x <= IGRPMODR31 as usize) ||
                    (x >= NSACR0 as usize && x <= SIGR as usize) || (x >= CPENDSGIR0 as usize && x <= SPENDSGIR3 as usize) ||
                    (x >= IROUTER32 as usize && x <= IROUTER1019 as usize)
            }
        }
    }
}

/// In the spec, these register names are prefixed with "GICD_".
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
#[allow(non_camel_case_types)]
enum DistMmio {
    CTLR          = 0x0000,
    TYPER         = 0x0004,
    IIDR          = 0x0008,
    STATUSR       = 0x0010,
    SETSPI_NSR    = 0x0040,
    CLRSPI_NSR    = 0x0048,
    SETSPI_SR     = 0x0050,
    CLRSPI_SR     = 0x0058,
    IGROUPR0      = 0x0080, /* ... */ IGROUP31      = 0x00fc,
    ISENABLER0    = 0x0100, /* ... */ ISENABLER31   = 0x017c,
    ICENABLER0    = 0x0180, /* ... */ ICENABLER31   = 0x01fc,
    ISPENDR0      = 0x0200, /* ... */ ISPENDR31     = 0x027c,
    ICPENDR0      = 0x0280, /* ... */ ICPENDR31     = 0x02fc,
    ISACTIVER0    = 0x0300, /* ... */ ISACTIVER31   = 0x037c,
    ICACTIVER0    = 0x0380, /* ... */ ICACTIVER31   = 0x03fc,
    IPRIORITYR0   = 0x0400, /* ... */ IPRIORITYR254 = 0x07f8,
    ITARGETSR0    = 0x0800, /* ... */ ITARGETSR254  = 0x0bf8,
    ICFGR0        = 0x0c00, /* ... */ ICFGR63       = 0x0cfc,
    IGRPMODR0     = 0x0d00, /* ... */ IGRPMODR31    = 0x0d7c,
    NSACR0        = 0x0e00, /* ... */ NSACR63       = 0x0efc,
    SIGR          = 0x0f00,
    CPENDSGIR0    = 0x0f10, /* ... */ CPENDSGIR3    = 0x0f1c,
    SPENDSGIR0    = 0x0f20, /* ... */ SPENDSGIR3    = 0x0f2c,
    IROUTER32     = 0x6100, /* ... */ IROUTER1019   = 0x7fd8,
    PIDR2         = 0xffe8
}

impl CpuRegs {
    fn new(base: usize, size: usize) -> Self {
        if let Ok(block) = AllMemAlloc.mmio_mut(base, size) {
            let mut regs = CpuRegs {
                block,
                arch_version: 0
            };
            regs.arch_version = unsafe { GiccIidr::from_bits_truncate(regs.read_mmio(CpuMmio::IIDR)).arch_version() };
            regs
        } else {
            panic!("{}", Text::GicCouldntReserveCpuIntBlock);
        }
    }

    /// Reads the value of the given register. It might read from MMIO or from a system register.
    pub fn read(&self, reg: CpuMmio) -> u32 {
        if !Self::is_reg_readable(reg) {
            panic!("{}", Text::GicReadUnreadableCpuIntReg(reg as usize));
        }

        // Read from a system register if possible.
        unsafe {
            if let Ok(value) = self.read_sysreg(reg) {
                value
            } else {
                self.read_mmio(reg)
            }
        }
    }

    /// Writes the given value to the given register.
    pub fn write(&self, reg: CpuMmio, val: u32) {
        if !Self::is_reg_writable(reg) {
            panic!("{}", Text::GicWriteUnwritableCpuIntReg(reg as usize));
        }

        unsafe {
            if self.write_sysreg(reg, val).is_err() {
                self.write_mmio(reg, val);
            }
        }
    }

    /// Enables interrupts on this interface.
    pub fn enable(&self) {
        // Before enabling interrupts, set the priority mask to the lowest possible priority to
        // make sure we accept all of them.
        let pmr = self.read(CpuMmio::PMR);
        self.write(CpuMmio::PMR, pmr | 0xff);

        let icc_sre = self.get_icc_sre();
        if icc_sre.contains(IccSre::SRE) {
            unsafe {
                // The system register interface has three registers for the initialization
                // we're doing.
                self.set_icc_sre(icc_sre
                    | IccSre::DFB // Disable FIQ and IRQ bypass. If there's a GIC, we don't support anything else.
                    | IccSre::DIB
                );
                #[cfg(target_arch = "aarch64")] {
                    let control_flags: u32;
                    asm!("mrs {:x}, ICC_CTLR_EL1", out(reg) control_flags, options(nomem, nostack, preserves_flags));
                    let control_flags = IccCtlr::from_bits(control_flags).unwrap()
                        | IccCtlr::PMHE // Priority masks used as hints for IRQ routing
                        & !IccCtlr::EOI_MODE; // EOI both deactivates IRQ and drops running priority
                    asm!("msr ICC_CTLR_EL1, {:x}", in(reg) control_flags.bits(), options(nomem, nostack, preserves_flags));

                    // TODO: Is there a better way to find out whether we're in Secure mode?
                    let group_enable: u32;
                    asm!("mrs {:x}, ICC_IGRPEN0_EL1", out(reg) group_enable, options(nomem, nostack, preserves_flags));
                    let group_enable = IccIGrpEn::from_bits(group_enable).unwrap()
                        | IccIGrpEn::ENABLE; // Enable Group 1 (i.e. non-Secure) interrupts
                    asm!("msr ICC_IGRPEN0_EL1, {:x}", in(reg) group_enable.bits(), options(nomem, nostack, preserves_flags));
                } #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))] {
                    let control_flags: u32;
                    asm!("mrs {}, ICC_CTLR", out(reg) control_flags, options(nomem, nostack, preserves_flags));
                    let control_flags = IccCtlr::from_bits(control_flags).unwrap()
                        | IccCtlr::PMHE // Priority masks used as hints for IRQ routing
                        & !IccCtlr::EOI_MODE; // EOI both deactivates IRQ and drops running priority
                    asm!("msr ICC_CTLR, {}", in(reg) control_flags.bits(), options(nomem, nostack, preserves_flags));

                    let group_enable: u32;
                    asm!("mrs {}, ICC_IGRPEN0", out(reg) group_enable, options(nomem, nostack, preserves_flags));
                    let group_enable = IccIGrpEn::from_bits(group_enable).unwrap()
                        | IccIGrpEn::ENABLE; // Enable interrupts
                    asm!("msr ICC_IGRPEN0, {}", in(reg) group_enable.bits(), options(nomem, nostack, preserves_flags));
                }
            }
        } else {
            // Much simpler legacy interface. Everything we need is in one register.
            unsafe {
                let ctlr = GiccCtlr::from_bits_truncate(self.read_mmio(CpuMmio::CTLR));
                self.write_mmio(CpuMmio::CTLR, (ctlr
                    | GiccCtlr::ENABLE_GRP_0
                    | GiccCtlr::FIQ_BYPASS_DISABLE
                    | GiccCtlr::IRQ_BYPASS_DISABLE
                    & !GiccCtlr::EOI_MODE
                ).bits() | 2);
            }
        }
    }

    /// Reads the value of the given register, using MMIO regardless of whether it's available as a
    /// system register. This should only be used when MMIO is known to be correct.
    ///
    /// # Safety
    /// This function is `unsafe` because, if misused, it leads to undefined behavior. A board that maps
    /// the given GIC register to a system register is not required to offer it via MMIO, and if it does,
    /// it is not required to have both mappings refer to the same physical register.
    unsafe fn read_mmio(&self, reg: CpuMmio) -> u32 {
        // For some reason, QEMU doesn't support the required GICC_IIDR register. This special case
        // will have to do for now.
        #[cfg(target_machine = "qemu-virt")] {
            if let CpuMmio::IIDR = reg {
                return 0x0002_0000;
            }
        }

        (&*(self.block.index(reg as usize) as *mut _ as *mut Volatile<u32>)).read()
    }

    /// Writes the given value to the given register, using MMIO regardless of whether it's available as
    /// a system register. This should only be used when MMIO is known to be correct.
    ///
    /// # Safety
    /// This function is `unsafe` because, if misused, it leads to undefined behavior. A board that maps
    /// the given GIC register to a system register is not required to offer it via MMIO, and if it does,
    /// it is not required to have both mappings refer to the same physical register.
    unsafe fn write_mmio(&self, reg: CpuMmio, val: u32) {
        (&mut *(self.block.index(reg as usize) as *mut _ as *mut Volatile<u32>)).write(val);
    }

    /// Reads the value of the given register as a system register, if it can be accessed in that way.
    ///
    /// # Safety
    /// This function is `unsafe` because, if misused, it leads to undefined behavior. A board that maps
    /// GIC registers to both MMIO and system registers is not required to actually use the same physical
    /// register for both mappings.
    ///
    /// # Returns
    /// `Ok(v)`, where `v` is the value of the register, or `Err(())` if it's not readable as a system
    /// register.
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    unsafe fn read_sysreg(&self, reg: CpuMmio) -> Result<u32, ()> {
        if !self.get_icc_sre().contains(IccSre::SRE) {
            return Err(());
        }

        let val: u32;
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))] {
            match reg {
                CpuMmio::PMR => {
                        asm!("mrc p15, 0, {}, c4, c6, 0", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::BPR0 => {
                        asm!("mrc p15, 0, {}, c12, c8, 3", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::IAR0 => {
                        asm!("mrc p15, 0, {}, c12, c8, 0", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::RPR => {
                        asm!("mrc p15, 0, {}, c12, c11, 3", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::HPPIR0 => {
                        asm!("mrc p15, 0, {}, c12, c8, 2", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::BPR1 => {
                        asm!("mrc p15, 0, {}, c12, c12, 3", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::IAR1 => {
                        asm!("mrc p15, 0, {}, c12, c12, 0", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::HPPIR1 => {
                        asm!("mrc p15, 0, {}, c12, c12, 2", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R0 => {
                        asm!("mrc p15, 0, {}, c12, c8, 4", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R1 => {
                        asm!("mrc p15, 0, {}, c12, c8, 5", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R2 => {
                        asm!("mrc p15, 0, {}, c12, c8, 6", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R3 => {
                        asm!("mrc p15, 0, {}, c12, c8, 7", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R0 => {
                        asm!("mrc p15, 0, {}, c12, c9, 0", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R1 => {
                        asm!("mrc p15, 0, {}, c12, c9, 1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R2 => {
                        asm!("mrc p15, 0, {}, c12, c9, 2", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R3 => {
                        asm!("mrc p15, 0, {}, c12, c9, 3", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                _ => Err(())
            }
        } #[cfg(target_arch = "aarch64")] {
            match reg {
                CpuMmio::PMR => {
                        asm!("mrs {:x}, ICC_PMR_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::BPR0 => {
                        asm!("mrs {:x}, ICC_BPR0_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::IAR0 => {
                        asm!("mrs {:x}, ICC_IAR0_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::RPR => {
                        asm!("mrs {:x}, ICC_RPR_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::HPPIR0 => {
                        asm!("mrs {:x}, ICC_HPPIR0_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::BPR1 => {
                        asm!("mrs {:x}, ICC_BPR1_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::IAR1 => {
                        asm!("mrs {:x}, ICC_IAR1_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::HPPIR1 => {
                        asm!("mrs {:x}, ICC_HPPIR1_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R0 => {
                        asm!("mrs {:x}, ICC_AP0R0_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R1 => {
                        asm!("mrs {:x}, ICC_AP0R1_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R2 => {
                        asm!("mrs {:x}, ICC_AP0R2_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP0R3 => {
                        asm!("mrs {:x}, ICC_AP0R3_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R0 => {
                        asm!("mrs {:x}, ICC_AP1R0_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R1 => {
                        asm!("mrs {:x}, ICC_AP1R1_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R2 => {
                        asm!("mrs {:x}, ICC_AP1R2_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                CpuMmio::AP1R3 => {
                        asm!("mrs {:x}, ICC_AP1R3_EL1", out(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(val)
                    },
                _ => Err(())
            }
        }
    }

    /// Writes the given value to the given register as a system register, if it can be accessed in that way.
    ///
    /// # Safety
    /// This function is `unsafe` because, if misused, it leads to undefined behavior. A board that maps
    /// GIC registers to both MMIO and system registers is not required to actually use the same physical
    /// register for both mappings.
    ///
    /// # Returns
    /// `Ok(())`, or `Err(())` if it's not writable as a system register.
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    unsafe fn write_sysreg(&self, reg: CpuMmio, val: u32) -> Result<(), ()> {
        if !self.get_icc_sre().contains(IccSre::SRE) {
            return Err(());
        }

        let val = val;
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))] {
            match reg {
                CpuMmio::PMR => {
                        asm!("mcr p15, 0, {}, c4, c6, 0", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::BPR0 => {
                        asm!("mcr p15, 0, {}, c12, c8, 3", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::EOIR0 => {
                        asm!("mcr p15, 0, {}, c12, c8, 1", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::BPR1 => {
                        asm!("mcr p15, 0, {}, c12, c12, 3", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::EOIR1 => {
                        asm!("mcr p15, 0, {}, c12, c12, 1", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::DIR => {
                        asm!("mcr p15, 0, {}, c12, c11, 1", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R0 => {
                        asm!("mcr p15, 0, {}, c12, c8, 4", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R1 => {
                        asm!("mcr p15, 0, {}, c12, c8, 5", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R2 => {
                        asm!("mcr p15, 0, {}, c12, c8, 6", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R3 => {
                        asm!("mcr p15, 0, {}, c12, c8, 7", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R0 => {
                        asm!("mcr p15, 0, {}, c12, c9, 0", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R1 => {
                        asm!("mcr p15, 0, {}, c12, c9, 1", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R2 => {
                        asm!("mcr p15, 0, {}, c12, c9, 2", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R3 => {
                        asm!("mcr p15, 0, {}, c12, c9, 3", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                _ => Err(())
            }
        } #[cfg(target_arch = "aarch64")] {
            match reg {
                CpuMmio::PMR => {
                        asm!("msr ICC_PMR_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::BPR0 => {
                        asm!("msr ICC_BPR0_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::EOIR0 => {
                        asm!("msr ICC_EOIR0_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::BPR1 => {
                        asm!("msr ICC_BPR1_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::EOIR1 => {
                        asm!("msr ICC_EOIR1_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::DIR => {
                        asm!("msr ICC_DIR_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R0 => {
                        asm!("msr ICC_AP0R0_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R1 => {
                        asm!("msr ICC_AP0R1_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R2 => {
                        asm!("msr ICC_AP0R2_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP0R3 => {
                        asm!("msr ICC_AP0R3_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R0 => {
                        asm!("msr ICC_AP1R0_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R1 => {
                        asm!("msr ICC_AP1R1_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R2 => {
                        asm!("msr ICC_AP1R2_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                CpuMmio::AP1R3 => {
                        asm!("msr ICC_AP1R3_EL1, {:x}", in(reg) val, options(nomem, nostack, preserves_flags));
                        Ok(())
                    },
                _ => Err(())
            }
        }
    }

    fn is_reg_readable(reg: CpuMmio) -> bool {
        if reg as usize % 4 != 0 {
            return false;
        }
        match reg {
            CpuMmio::CTLR |
            CpuMmio::PMR |
            CpuMmio::BPR0 |
            CpuMmio::IAR0 |
            CpuMmio::RPR |
            CpuMmio::HPPIR0 |
            CpuMmio::BPR1 |
            CpuMmio::IAR1 |
            CpuMmio::HPPIR1 |
            CpuMmio::STATUSR |
            CpuMmio::IIDR => true,
            _ => {
                let reg = reg as usize;
                (reg >= CpuMmio::AP0R0 as usize && reg <= CpuMmio::AP0R3 as usize) ||
                    (reg >= CpuMmio::AP1R0 as usize && reg <= CpuMmio::AP1R3 as usize)
            }
        }
    }

    fn is_reg_writable(reg: CpuMmio) -> bool {
        if reg as usize % 4 != 0 {
            return false;
        }
        match reg {
            CpuMmio::CTLR |
            CpuMmio::PMR |
            CpuMmio::BPR0 |
            CpuMmio::EOIR0 |
            CpuMmio::BPR1 |
            CpuMmio::EOIR1 |
            CpuMmio::STATUSR |
            CpuMmio::DIR => true,
            _ => {
                let reg = reg as usize;
                (reg >= CpuMmio::AP0R0 as usize && reg <= CpuMmio::AP0R3 as usize) ||
                    (reg >= CpuMmio::AP1R0 as usize && reg <= CpuMmio::AP1R3 as usize)
            }
        }
    }

    /// Sets the value of the ICC_SRE_EL1 system register (ICC_SRE in Aarch32).
    /// This determines, among other things, whether system registers or memory-mapped I/O should be used
    /// for the CPU interface.
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    fn set_icc_sre(&self, value: IccSre) {
        // This register was introduced in GICv3. We can pretend it's RAZ/WI if it doesn't exist.
        if self.arch_version < 3 {
            return;
        }

        let value: u32 = value.bits();
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
        unsafe {
            asm!("mcr p15, 0, {}, c12, c12, 5", in(reg) value, options(nomem, nostack, preserves_flags));
        }
        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!("msr ICC_SRE_EL1, {:x}", in(reg) value, options(nomem, nostack, preserves_flags));
        }
    }

    /// Gets the value of the ICC_SRE_EL1 system register (ICC_SRE in Aarch32).
    /// This indicates, among other things, whether system registers or memory-mapped I/O should be used.
    #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7", target_arch = "aarch64"))]
    fn get_icc_sre(&self) -> IccSre {
        // This register was introduced in GICv3. We can pretend it's RAZ/WI if it doesn't exist.
        if self.arch_version < 3 {
            return IccSre::empty();
        }

        let value: u32;
        #[cfg(any(target_arch = "arm", target_arch = "armv5te", target_arch = "armv7"))]
        unsafe {
            asm!("mrc p15, 0, {}, c12, c12, 5", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        #[cfg(any(target_arch = "aarch64"))]
        unsafe {
            asm!("mrs {:x}, ICC_SRE_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        IccSre::from_bits_truncate(value)
    }
}

/// In the spec, these register names are prefixed with "GICC_".
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum CpuMmio {
    // Don't use these directly. The bitmap changes based on how and when GICC_CTLR is accessed.
    CTLR    = 0x0000,
    // IGRPEN0 = 0x0000,
    // IGRPEN1 = 0x0000,

    PMR     = 0x0004,
    BPR0    = 0x0008,
    IAR0    = 0x000c,
    EOIR0   = 0x0010,
    RPR     = 0x0014,
    HPPIR0  = 0x0018,
    BPR1    = 0x001c, // a.k.a. ABPR
    IAR1    = 0x0020, // a.k.a. AIAR
    EOIR1   = 0x0024, // a.k.a. AEOIR
    HPPIR1  = 0x0028, // a.k.a. AHPPIR
    STATUSR = 0x002c,
    AP0R0   = 0x00d0,
    AP0R1   = 0x00d4,
    AP0R2   = 0x00d8,
    AP0R3   = 0x00dc,
    AP1R0   = 0x00e0, // a.k.a. NSAPR0
    AP1R1   = 0x00e4, // a.k.a. NSAPR1
    AP1R2   = 0x00e8, // a.k.a. NSAPR2
    AP1R3   = 0x00ec, // a.k.a. NSAPR3
    IIDR    = 0x00fc,
    DIR     = 0x1000

    // These registers are only available as system registers:
    // SGI0R
    // SGI1R
    // ASGI1R
    // SRE
}

struct Gic {
    pub dist_regs: DistRegs,
    pub cpu_regs: CpuRegs
}

lazy_static! {
    unsafe {
        static ref GIC: Gic = init();
    }
}

lazy_static! {
    unsafe {
        // The GIC revisions to which the parts of the GIC conform (they may all be different), and also
        // some implementation-defined information stored in the same registers.
        static ref GICD_PIDR2: GicdPidr2 = GicdPidr2::from_bits_truncate(GIC.dist_regs.read(DistMmio::PIDR2));
    }
}

bitflags! {
    struct IccSre: u32 {
        const SRE = 0x1; // System register enable
        const DFB = 0x2; // Disable FIQ bypass
        const DIB = 0x4; // Disable IRQ bypass
    }
}

bitflags! {
    struct GicdCtlr: u32 {
        const ENABLE_GRP_0    = 0x01; // Enable Group 0 interrupts
        const ENABLE_GRP_1_NS = 0x02; // Enable Non-Secure Group 1 interrupts
        const ENABLE_GRP_1_S  = 0x04; // Enable Secure Group 1 interrupts
        const ARE_S           = 0x10; // Enable affinity routing for Secure state
        const ARE_NS          = 0x20; // Enable affinity routing for Non-Secure state
        const DS              = 0x40; // Disable security (can't be cleared without a hard reset)
        const E1NWF           = 0x80; // Enable "1 of N Wakeup Functionality": a sleeping PE can be woken up for a 1 of N interrupt
        const RWP             = 0x80000000; // Register Write Pending (read-only)
        const RESERVED        = 0x7fffff08;
    }
}

bitflags! {
    struct IccCtlr: u32 {
        const CBPR     = 0x00001;
        const EOI_MODE = 0x00002;
        const PMHE     = 0x00040;
        const PRI_BITS = 0x00700;
        const ID_BITS  = 0x03800;
        const SEIS     = 0x04000;
        const A3V      = 0x08000;
        const RSS      = 0x40000;
        const RESERVED = 0xfffb_00bc;
    }
}

bitflags! {
    struct GiccCtlr: u32 {
        const ENABLE_GRP_0       = 0x001; // Actually enables Group 1 in Non-Secure mode
        const FIQ_BYPASS_DISABLE = 0x020;
        const IRQ_BYPASS_DISABLE = 0x040;
        const EOI_MODE           = 0x200;
        const RESERVED           = 0xffff_fd9e; // TODO: Some of these are actually defined in the Secure view.
    }
}

bitflags! {
    struct IccIGrpEn: u32 {
        const ENABLE   = 0x1;
        const RESERVED = 0xffff_fffe;
    }
}

bitflags! {
    struct GicdPidr2: u32 {
        const ARCH_VERSION = 0xf0;
        const IMP_DEFINED  = 0xffff_ff0f;
    }
}

/*impl GicdPidr2 {
    pub fn arch_version(&self) -> u32 {
        (*self & Self::ARCH_VERSION).bits() >> 4
    }
}*/

bitflags! {
    struct GiccIidr: u32 {
        const IMPLEMENTOR  = 0x0000_0fff;
        const REVISION     = 0x0000_f000;
        const ARCH_VERSION = 0x000f_0000;
        const PRODUCT_ID   = 0xfff0_0000;
    }
}

impl GiccIidr {
    pub fn arch_version(&self) -> u32 {
        (*self & Self::ARCH_VERSION).bits() >> 16
    }
}

/// Returns the highest IRQ number that is supported by this GIC.
pub fn max_irq() -> u64 {
    // TODO: Use bitflags to enumerate the fields of GICD_TYPER.
    // TODO: Cache this value? It's probably not worth the effort. Maybe with a lazy static.
    let calculated = 32 * (((GIC.dist_regs.read(DistMmio::TYPER) & 0x1f) + 1) - 1) as u64;
    if calculated < 1019 { calculated } else { 1019 }
}

/// Returns the number of CPUs that can receive interrupts if affinity routing isn't enabled.
fn cpu_count_without_affinity_routing() -> u8 {
    // TODO: Use bitflags to enumerate the fields of GICD_TYPER.
    // TODO: Cache this value? It's probably not worth the effort. Maybe with a lazy static.
    (((GIC.dist_regs.read(DistMmio::TYPER) & 0xe0) >> 5) + 1) as u8
}

/// Initializes the Generic Interrupt Controller.
fn init() -> Gic {
    #[cfg(target_machine = "qemu-virt")]
    let gic = Gic {
        dist_regs: DistRegs::new(0x0800_0000, 0x0001_0000),
        cpu_regs:   CpuRegs::new(0x0801_0000, 0x0001_0000)
    };

    // Enable using the system registers to access the GIC registers that support it.
    gic.cpu_regs.set_icc_sre(IccSre::SRE);
    if gic.cpu_regs.get_icc_sre().contains(IccSre::SRE) {
        printlndebug!("Setting ICC_SRE succeeded. Using system registers.");
    } else {
        printlndebug!("Setting ICC_SRE failed. Using memory-mapped registers.");
    }

    // TODO: Do this for each CPU.
    init_cpu_interface(&gic.cpu_regs);

    init_distributor(&gic.dist_regs);

    // TODO: In order to wake the other CPUs, we probably have to send them an SGI
    // (software-generated interrupt). See the GIC spec for details on how to do that.
    // In general, it depends on the initial state of the CPUs, which is implementation-
    // defined. Also, it probably doesn't make sense to wake the other CPUs in this
    // function.

    gic
}

/// Initializes the distributor.
fn init_distributor(dist_regs: &DistRegs) {
    let mut flags;

    loop {
        flags = GicdCtlr::from_bits_truncate(dist_regs.read(DistMmio::CTLR));
        if !flags.contains(GicdCtlr::RWP) { break; }
    }
    flags = flags // Temporarily disable sending interrupts to CPUs.
        & !GicdCtlr::ENABLE_GRP_0
        & !GicdCtlr::ENABLE_GRP_1_NS;
    dist_regs.write(DistMmio::CTLR, flags.bits());

    loop {
        flags = GicdCtlr::from_bits_truncate(dist_regs.read(DistMmio::CTLR));
        if !flags.contains(GicdCtlr::RWP) { break; }
    }
    flags |= GicdCtlr::ARE_NS | GicdCtlr::E1NWF; // Enable affinity routing and 1 of N waking.
    dist_regs.write(DistMmio::CTLR, flags.bits());

    loop {
        flags = GicdCtlr::from_bits_truncate(dist_regs.read(DistMmio::CTLR));
        if !flags.contains(GicdCtlr::RWP) { break; }
    }
    flags = flags // Enable sending interrupts to CPUs.
        | GicdCtlr::ENABLE_GRP_0
        | GicdCtlr::ENABLE_GRP_1_NS;
    dist_regs.write(DistMmio::CTLR, flags.bits());
}

/// Initializes the CPU interface for this CPU.
fn init_cpu_interface(cpu_regs: &CpuRegs) {
    cpu_regs.enable();
}
