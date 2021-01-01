// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2019-2021 Andre Richter <andre.o.richter@gmail.com>

//! Page faults must result in synchronous exceptions.

#![feature(format_args_nl)]
#![no_main]
#![no_std]

/// Overwrites libkernel's `panic_wait::_panic_exit()` with the QEMU-exit version.
///
/// Reaching this code is a success, because it is called from the synchronous exception handler,
/// which is what this test wants to achieve.
///
/// It also means that this integration test can not use any other code that calls panic!() directly
/// or indirectly.
mod panic_exit_success;

use libkernel::{bsp, cpu, exception, memory, println};

#[no_mangle]
unsafe fn kernel_init() -> ! {
    use libkernel::driver::interface::DriverManager;

    bsp::console::qemu_bring_up_console();

    println!("Testing synchronous exception handling by causing a page fault");
    println!("-------------------------------------------------------------------\n");

    exception::handling_init();

    if let Err(string) = memory::mmu::kernel_map_binary_and_enable_mmu() {
        println!("Enabling MMU failed: {}", string);
        cpu::qemu_exit_failure()
    }
    // Printing will silently fail fail from here on, because the driver's MMIO is not remapped yet.

    // Bring up the drivers needed for printing first.
    for i in bsp::driver::driver_manager()
        .early_print_device_drivers()
        .iter()
    {
        // Any encountered errors cannot be printed yet, obviously, so just safely park the CPU.
        i.init().unwrap_or_else(|_| cpu::qemu_exit_failure());
    }
    bsp::driver::driver_manager().post_early_print_device_driver_init();
    // Printing available again from here on.

    println!("Writing beyond mapped area to address 9 GiB...");
    let big_addr: u64 = 9 * 1024 * 1024 * 1024;
    core::ptr::read_volatile(big_addr as *mut u64);

    // If execution reaches here, the memory access above did not cause a page fault exception.
    cpu::qemu_exit_failure()
}
