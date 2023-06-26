mod device;

use device::device_handler::DeviceHandler;

fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

fn main() {
    init_logger();
    
    let mut device_handler = DeviceHandler::new();
    device_handler.open().unwrap();
    device_handler.init().unwrap();
    device_handler.program(std::path::Path::new("./tests/bitfile/Dino_fde_yosys.bit")).unwrap();
    device_handler.close().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open() {
        let mut device_handler = DeviceHandler::new();
        let result = device_handler.open();

        if let Err(e) = result {
            println!("Error: {}", e);
        }
    }
}
