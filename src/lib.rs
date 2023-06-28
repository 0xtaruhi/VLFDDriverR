mod device;

use std::os::raw::*;

#[no_mangle]
pub extern "C" fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[no_mangle]
pub extern "C" fn program(bitfile: *const c_char) -> c_int {
    let mut device_handler = device::device_handler::DeviceHandler::new();

    let result = device_handler.open();

    if let Err(e) = result {
        println!("Error: {}", e);
        return 1;
    }

    let result = device_handler.init();

    if let Err(e) = result {
        println!("Error: {}", e);
        return 1;
    }

    let result = device_handler.program(std::path::Path::new(unsafe {
        std::ffi::CStr::from_ptr(bitfile).to_str().unwrap()
    }));

    if let Err(e) = result {
        println!("Error: {}", e);
        return 1;
    }

    let result = device_handler.close();

    if let Err(e) = result {
        println!("Error: {}", e);
        return 1;
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
