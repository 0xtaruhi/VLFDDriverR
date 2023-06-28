mod device;

use std::{os::raw::*};
use device::program_handler::ProgramHandler;

#[no_mangle]
pub extern "C" fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[no_mangle]
pub extern "C" fn program(bitfile: *const c_char) -> c_int {
    let mut program_handler = ProgramHandler::new();

    // Open device
    let result = program_handler.open_device();
    if result.is_err() {
        return -1;
    }

    // Program
    let result = program_handler.program(
        std::path::Path::new(
            unsafe { std::ffi::CStr::from_ptr(bitfile) }
                .to_str()
                .unwrap(),
        ),
    );
    if result.is_err() {
        return -1;
    }

    // Close device
    let result = program_handler.close_device();
    if result.is_err() {
        return -1;
    }
    
    return 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program() {
        init_logger();
        let result = program(
            std::ffi::CString::new("tests/bitfile/Name_fde_yosys.bit")
                .unwrap()
                .into_raw(),
        );
        assert_eq!(result, 0);
    }
}
