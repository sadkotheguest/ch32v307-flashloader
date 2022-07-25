#![no_std]
#![no_main]
// Define necessary functions for flash loader
//
// These are taken from the [ARM CMSIS-Pack documentation]
//
// [ARM CMSIS-Pack documentation]: https://arm-software.github.io/CMSIS_5/Pack/html/algorithmFunc.html

use core::slice;
use ch32v307_pac::{FLASH, RCC};
use panic_abort as _;

/// Segger tools require the PrgData section to exist in the target binary
///
/// They also scan the flashloader binary for this symbol to determine the section location
/// If they cannot find it, the tool exits. This variable serves no other purpose.
#[allow(non_upper_case_globals)]
#[no_mangle]
#[used]
#[link_section = "PrgData"]
pub static PRGDATA_Start: usize = 0;

/// Erase the sector at the given address in flash
///
/// `Return` - 0 on success, 1 on failure.
#[no_mangle]
#[inline(never)]
pub extern "C" fn EraseSector(adr: u32) -> i32 {

    // TODO: Code UnInit for CH32V307

    // ----- [vvv] Example for GD32VF103 -----
    let fmc = unsafe { &(*FMC::ptr()) };
    // Enable "Page erase"
    fmc.ctl0.write_with_zero(|w| w.per().set_bit());
    // Sector address
    fmc.addr0.write(|w| unsafe { w.addr().bits(adr) });
    // Start erase operation
    fmc.ctl0.modify(|_, w| w.start().set_bit());
    while fmc.stat0.read().busy().bit_is_set() {
        // TODO: feed watchdog
    }
    // Disable "Page erase"
    fmc.ctl0.write_with_zero(|w| w.per().clear_bit());

    // On error, clear all the bits except for the error bits and return failure
    if fmc.stat0.read().pgerr().bit_is_set() || fmc.stat0.read().wperr().bit_is_set() {
        fmc.stat0
            .write_with_zero(|w| w.pgerr().set_bit().wperr().set_bit());
        1
    } else {
        0
    }
}

/// Initializes the microcontroller for Flash programming. Returns 0 on Success, 1 otherwise
///
/// This is invoked whenever an attempt is made to download the program to Flash.
///
///  # Arguments
///
/// `adr` - specifies the base address of the device.
///
/// `clk` - specifies the clock frequency for prgramming the device.
///
/// `fnc` - is a number: 1=Erase, 2=Program, 3=Verify, to perform different init based on command
#[no_mangle]
#[inline(never)]
pub extern "C" fn Init(_adr: u32, _clk: u32, _fnc: u32) -> i32 {
    // C firmware saved the state of the clock and flash controller to static variables.
    // We're going to leave the clocks set up set the flash back to reset values on exit instead
    // Maybe deal with that later.
    let rcc = unsafe { &(*RCC::ptr()) };
    let flash = unsafe{ &(*FLASH::ptr()) };

    // init PLL to 96MHz (maximum FLASH operation frequency is 100 MHz)

    // TODO: Code UnInit for CH32V307

    // ----- [vvv] Example for GD32VF103 -----
    let rcu = unsafe { &(*RCU::ptr()) };
    let fmc = unsafe { &(*FMC::ptr()) };

    // init PLL to 108MHz
    rcu.ctl.modify(|_, w| w.irc8men().set_bit()); // enable IRC8 clock
    while rcu.ctl.read().irc8mstb().bit_is_clear() {} // wait till clock is stable
    rcu.cfg0.modify(|_, w| unsafe { w.scs().bits(0b00) }); // set IRC8M as CK_SYS source
    while rcu.cfg0.read().scss().bits() != 0b00 {} // wait till clock has been selected
    rcu.ctl.modify(|_, w| w.pllen().clear_bit()); // disable PLL
    rcu.cfg0.modify(|_, w| unsafe {
        w.ahbpsc().bits(0b0000); // set AHB prescaler to 1
        w.apb1psc().bits(0b100); // set APB1 prescaler to 2
        w.apb2psc().bits(0b000); // set APB2 prescaler to 1
        w.pllsel().clear_bit(); // use IRC8M/2 as PLL input
        w.pllmf_4().set_bit(); // set multiplier to 27 (0b11010)
        w.pllmf_3_0().bits(0b1010); // lower bits of multiplier
        w
    });
    rcu.ctl.modify(|_, w| w.pllen().set_bit()); // enable PLL
    while rcu.ctl.read().pllstb().bit_is_clear() {} // wait until PLL is stable
    rcu.cfg0.modify(|_, w| unsafe { w.scs().bits(0b10) }); // set PLL as CK_SYS source
    while rcu.cfg0.read().scss().bits() != 0b10 {} // wait until clock has been selected

    // Unlock flash bank 0
    if fmc.ctl0.read().lk().bit_is_set() {
        const FLASH_KEY1: u32 = 0x45670123;
        const FLASH_KEY2: u32 = 0xCDEF89AB;

        for key in [FLASH_KEY1, FLASH_KEY2] {
            fmc.key0.write(|w| unsafe { w.bits(key) })
        }
    }

    0
}

#[no_mangle]
#[inline(never)]
pub extern "C" fn ProgramPage(adr: u32, sz: u32, buf: *const u8) -> i32 {

    // TODO: Code UnInit for CH32V307

    // ----- [vvv] Example for GD32VF103 -----    

    let fmc = unsafe { &(*FMC::ptr()) };
    // Set page write
    fmc.ctl0.write_with_zero(|w| w.pg().set_bit());
    // Should we assume usize programming?
    // It's what the C code did, its fast and may be required, but oh-so-unsafe...
    let adr = adr as usize;
    let sz = sz as usize;
    let sz_usize = sz >> 2; // u32 = 4 bytes, right-shift by 2 is equivalent
    let buf_usize = buf as *const usize;
    // At least get one bit of usability out of rust.
    // Trying to avoid the provenance debate on destination by constructing pointers from integers
    let src_slice = unsafe { slice::from_raw_parts(buf_usize, sz_usize) };
    for (offset, item) in src_slice.iter().enumerate().take(sz) {
        let dst = (adr + offset * 4) as *mut usize;

        unsafe { dst.write_volatile(*item) };
        while fmc.stat0.read().busy().bit_is_set() {
            // TODO: feed watchdog
        }
        // If there's a programming error or write-protect error
        if fmc.stat0.read().pgerr().bit_is_set() || fmc.stat0.read().wperr().bit_is_set() {
            // Lock flash
            fmc.key0.write(|w| unsafe { w.bits(0) });
            return 1;
        }
    }

    0
}

/// De-initializes the microcontroller after Flash programming. Returns 0 on Success, 1 otherwise
///
/// This is invoked at the end of an erasing, programming, or verifying step.
///
///  # Arguments
///
/// `fnc` - is a number: 1=Erase, 2=Program, 3=Verify, to perform different de-init based on command
#[no_mangle]
#[inline(never)]
pub extern "C" fn UnInit(_fnc: u32) -> i32 {

    // TODO: Code UnInit for CH32V307

    // ----- [vvv] Example for GD32VF103 -----

    let fmc = unsafe { &(*FMC::ptr()) };

    // We could de-initialize, but it's a lot of work.
    // Let's leave the clocks alone and only reset the flash controller.
    // Hopefully that's enough.
    fmc.ctl0.reset();
    0
}

const fn sectors() -> [FlashSector; 512] {
    let mut sectors = [FlashSector::default(); 512];

    // 1KB sectors starting at address 0
    sectors[0] = FlashSector {
        size: 0x0400,
        address: 0x0,
    };
    sectors[1] = SECTOR_END;

    sectors
}

#[allow(non_upper_case_globals)]
#[no_mangle]
#[link_section = "DeviceData"]
pub static FlashDevice: FlashDeviceDescription = FlashDeviceDescription {

    // ToDo: 

    // ----- [vvv] Example for GD32VF103 ----
    vers: 0x0101,
    // dev_name: "GD32VF103 128 KB internal flash"
    dev_name: [
        // These rows have 12 entries, 12x3 = 36 bytes - need 92 more
        0x47, 0x44, 0x33, 0x32, 0x56, 0x46, 0x31, 0x30, 0x33, 0x20, 0x31, 0x32, 0x38, 0x20, 0x4b,
        0x42, 0x20, 0x69, 0x6e, 0x74, 0x65, 0x72, 0x6e, 0x61, 0x6c, 0x20, 0x66, 0x6c, 0x61, 0x73,
        0x68, 0x00, 0x00, 0x00, 0x00, 0x00,
        // These are 36 entries each, 36x2 = 72, need 20 more
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // and here are those 20
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ],
    dev_type: 1,
    dev_addr: 0x08000000,
    device_size: 0x00020000,
    page_size: 1024,
    _reserved: 0,
    empty: 0xff,
    program_time_out: 100,
    erase_time_out: 6000,
    flash_sectors: sectors(),
};

#[repr(C)]
pub struct FlashDeviceDescription {
    vers: u16,
    dev_name: [u8; 128],
    dev_type: u16,
    dev_addr: u32,
    device_size: u32,
    page_size: u32,
    _reserved: u32,
    empty: u8,
    program_time_out: u32,
    erase_time_out: u32,

    flash_sectors: [FlashSector; 512],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FlashSector {
    size: u32,
    address: u32,
}

impl FlashSector {
    const fn default() -> Self {
        FlashSector {
            size: 0,
            address: 0,
        }
    }
}

const SECTOR_END: FlashSector = FlashSector {
    size: 0xffff_ffff,
    address: 0xffff_ffff,
};
