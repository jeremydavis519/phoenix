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

//! This module contains all the logic for dealing with IRQs in a way specific to how they are used
//! with a GIC.

use {
    core::{
        fmt::Debug,
        mem,
        sync::atomic::{AtomicU8, AtomicUsize, Ordering}
    },

    i18n::Text,
    io::printlndebug,

    crate::{IsrFn, IsrResult}
};

extern "Rust" {
    fn scheduling_timer_finished() -> bool;
}

/// A smart pointer to an interrupt service routine. When it is dropped, the ISR
/// is unregistered.
#[derive(Debug, PartialEq, Eq)]
#[must_use]
pub struct IsrPtr {
    irq: usize
}

/// Describes the priority of an IRQ as compared to other IRQs. Higher-priority IRQs can pre-empt
/// lower-priority IRQs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Priority {
    /// The highest priority possible. This should probably never be used.
    Highest = 0x00,
    /// The highest priority that should probably be used.
    High    = 0x20,
    /// Pre-empts most interrupts but not all.
    MedHigh = 0x40,
    /// The middle priority, which should be the default.
    Medium  = 0x70,
    /// Is pre-empted by most interrupts but not all.
    MedLow  = 0xa0,
    /// The lowest priority that should probably be used.
    Low     = 0xc0,
    /// The lowest priority possible. This should probably never be used.
    Lowest  = 0xe0
}

/// Describes how the hardware will signal a given IRQ.
#[derive(Debug)]
pub enum IrqTrigger {
    /// A level-sensitive IRQ remains pending as long as the device maintains the signal.
    /// Therefore, it can only be completed by interfacing with the device somehow.
    Level,

    /// An edge-sensitive IRQ remains pending until the software acknowledges it with the GIC. Use
    /// this trigger mode if the device doesn't have its own way to acknowledge IRQs.
    Edge
}

struct IsrPtrNode {
    // TODO: Turn this into a linked list to allow multiple ISRs for the same IRQ number.
    isr: AtomicOptionIsrFnPtr
}

// Provides atomic access to the equivalent of an `Option<IsrFn>`.
struct AtomicOptionIsrFnPtr {
    ptr: AtomicUsize // The function pointer as an integer
}

/*impl IsrPtr {
    fn deref(&self) -> IsrFn {
        ISR_PTR_NODES.nodes[self.irq].isr.load(Ordering::Acquire).unwrap()
    }
}*/

impl Drop for IsrPtr {
    fn drop(&mut self) {
        // FIXME: Only unregister the ISR if this is the last `IsrPtr` referencing it.
        // FIXME: Only disable the interrupt if this is the last ISR registered for it.

        // Disable the interrupt in the GIC if it's not already disabled.
        super::GIC.dist_regs.disable_irq(self.irq as usize);

        // Remove the handler.
        ISR_PTR_NODES.remove_node(self.irq);
    }
}

impl IsrPtrNode {
    pub const fn new() -> IsrPtrNode {
        IsrPtrNode {
            isr: AtomicOptionIsrFnPtr::new()
        }
    }
}

// The data structure used to store all of the IsrPtrNodes.
struct IsrPtrNodeHeap {
    // TODO: We may need more information here to allow multiple ISRs for each IRQ.
    nodes: [IsrPtrNode; 1019]
}

impl IsrPtrNodeHeap {
    // Inserts a node for the given IRQ/ISR pair and returns an error if the heap is full.
    fn insert_node(&self, irq: usize, isr: IsrFn) -> Result<IsrPtr, ()> {
        match self.nodes[irq].isr.compare_exchange(None, Some(isr), Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(IsrPtr { irq }),
            Err(existing) if existing == Some(isr) => Ok(IsrPtr { irq }),
            Err(_) => Err(())
        }
    }

    // Removes the node with the given IRQ.
    fn remove_node(&self, irq: usize) {
        self.nodes[irq].isr.store(None, Ordering::Release);
    }

    // Runs the ISRs registered with the given IRQ until the correct one is found.
    fn handle_irq(&self, irq: usize) -> IsrResult {
        // TODO: Make this able to handle multiple ISRs.
        if let Some(isr) = self.nodes[irq].isr.load(Ordering::Acquire) {
            // We have an ISR. Run it and see if it's correct.
            match isr() {
                IsrResult::Serviced => return IsrResult::Serviced,
                IsrResult::PreemptThread => return IsrResult::PreemptThread,
                IsrResult::WrongIsr => {}
            };
        }

        // None of the ISRs were correct.
        IsrResult::WrongIsr
    }
}

impl AtomicOptionIsrFnPtr {
    const NULL: usize = 0;

    pub const fn new() -> Self {
        AtomicOptionIsrFnPtr {
            ptr: AtomicUsize::new(Self::NULL)
        }
    }

    pub fn load(&self, order: Ordering) -> Option<IsrFn> {
        Self::from_raw(self.ptr.load(order))
    }

    pub fn store(&self, val: Option<IsrFn>, order: Ordering) {
        self.ptr.store(Self::to_raw(val), order)
    }

    pub fn compare_exchange(&self, current: Option<IsrFn>, new: Option<IsrFn>, success: Ordering, failure: Ordering)
            -> Result<Option<IsrFn>, Option<IsrFn>> {
        self.ptr.compare_exchange(Self::to_raw(current), Self::to_raw(new), success, failure)
            .map(|raw| Self::from_raw(raw)).map_err(|raw| Self::from_raw(raw))
    }

    fn to_raw(val: Option<IsrFn>) -> usize {
        match val {
            None => Self::NULL,
            Some(v) => unsafe { mem::transmute(v) }
        }
    }

    fn from_raw(raw: usize) -> Option<IsrFn> {
        if raw == Self::NULL {
            None
        } else {
            unsafe { Some(mem::transmute(raw)) }
        }
    }
}

impl From<Option<IsrFn>> for AtomicOptionIsrFnPtr {
    fn from(maybe_isr: Option<IsrFn>) -> Self {
        Self { ptr: AtomicUsize::new(Self::to_raw(maybe_isr)) }
    }
}

impl Debug for AtomicOptionIsrFnPtr {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        if let Some(ptr) = self.load(Ordering::Acquire) {
            write!(f, "Some({:p})", ptr as *const ())
        } else {
            write!(f, "None")
        }
    }
}

static ISR_PTR_NODES: IsrPtrNodeHeap = IsrPtrNodeHeap {
    nodes: [const { IsrPtrNode::new() }; 1019]
};

/// Registers the given ISR to handle the given IRQ.
///
/// # Returns
/// A unique handle that will be unregistered when it's dropped.
pub fn register_irq(irq: u64, isr: IsrFn, priority: Priority, trigger: IrqTrigger) -> Result<IsrPtr, ()> {
    let max_irq = super::max_irq();
    if irq > max_irq {
        panic!("{}", Text::GicIrqOutOfBounds(irq, max_irq));
    }
    let irq = irq as usize;

    let ptr = ISR_PTR_NODES.insert_node(irq, isr)?;

    // Set up the interrupt's priority and routing.
    super::GIC.dist_regs.write_byte(super::DistMmio::IPRIORITYR0, irq as usize, priority as u8);
    // TODO: let processor_number = super::GIC.redist_regs.read(super::RedistMmio::TYPER).processor_number();
    // ITARGETSR<n> is read-only for n = 0 to 7 (IRQs 0 to 31). Presumably, those ones will be
    // routed without any further intervention. But all other IRQs need to be told where to go.
    if irq >= 32 {
        static COUNTER: AtomicU8 = AtomicU8::new(0);
        super::GIC.dist_regs.write_byte(super::DistMmio::ITARGETSR0, irq as usize, // If affinity routing is disabled, set a random CPU as the target.
            1 << (COUNTER.fetch_add(1, Ordering::AcqRel) % super::cpu_count_without_affinity_routing()));
        // TODO: Use bitflags here.
        // TODO: Support for affinity routing isn't implemented yet.
        //       Does this next line belong inside or outside the if block?
        // super::GIC.dist_regs.route_irq(irq, 0x00000000_80000000); // If it's enabled, route to any participating CPU.
    }

    // Enable the interrupt in the GIC if it's not already enabled.
    super::GIC.dist_regs.enable_irq(irq, trigger);

    Ok(ptr)
}

/// The entry point for all IRQ handlers.
///
/// # Returns
/// * 0 on normal completion
/// * 1 if the thread should be pre-empted
#[no_mangle]
pub fn aarch64_handle_irq() -> u8 {
    // Getting the INTID (i.e. IRQ number) also acts as an acknowledgement of the interrupt.
    let icc_iar = ack_irq();
    let intid = (icc_iar & 0x00ffffff) as usize;

    // INTIDs above 1019 mean that we're in the wrong Security state to handle them, or perhaps
    // that the interrupts are spurrious. In any case, they can be ignored.
    if intid > 1019 {
        return 0;
    }

    // Find and execute the right ISR for this IRQ.
    let result = match ISR_PTR_NODES.handle_irq(intid) {
        IsrResult::Serviced => 0,
        IsrResult::PreemptThread => 1,
        IsrResult::WrongIsr => {
            printlndebug!("Could not handle IRQ {}: wrong ISR", intid);
            0
        }
    };

    send_eoi(icc_iar);

    if result == 1 {
        result
    } else {
        // If the scheduling timer went off while we were handling this IRQ, we should still
        // leave userspace.
        if unsafe { scheduling_timer_finished() } {
            printlndebug!("Scheduling timer finished during IRQ {}", intid);
            1
        } else {
            result
        }
    }
}

fn ack_irq() -> u32 { super::GIC.cpu_regs.read(super::CpuMmio::IAR0) }

fn send_eoi(icc_iar: u32) { super::GIC.cpu_regs.write(super::CpuMmio::EOIR0, icc_iar); }
