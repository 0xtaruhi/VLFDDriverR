use libusb1_sys as ffi;
use log::{error, info};

use core::cell::RefCell;
use std::io::BufRead;

use super::cfg::Cfg;
use super::device_error::DeviceError;

const VID: u16 = 0x2200;
const PID: u16 = 0x2008;

#[derive(Clone, Copy)]
enum EndPoint {
    EP2 = 0x02,
    EP4 = 0x04,
    EP6 = 0x86,
    EP8 = 0x88,
}

pub struct DeviceHandler {
    handle: *mut ffi::libusb_device_handle,
    encrypt_table: [u16; 32],
    encode_index: RefCell<usize>,
    decode_index: RefCell<usize>,
    cfg: Cfg,
}

type DeviceResult<T> = Result<T, DeviceError>;

impl DeviceHandler {
    pub fn new() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            encrypt_table: [0u16; 32],
            encode_index: RefCell::new(0),
            decode_index: RefCell::new(0),
            cfg: Cfg::new(),
        }
    }

    pub fn open(&mut self) -> DeviceResult<()> {
        unsafe {
            ffi::libusb_init(std::ptr::null_mut());
        }

        let handle =
            unsafe { ffi::libusb_open_device_with_vid_pid(std::ptr::null_mut(), VID, PID) };

        if handle.is_null() {
            error!("Device open failed");
            return Err(DeviceError::OpenError);
        } else {
            info!("Device opened");
        }

        self.handle = handle;
        Ok(())
    }

    pub fn close(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                ffi::libusb_close(self.handle);
            }
        }

        unsafe {
            ffi::libusb_exit(std::ptr::null_mut());
        }
    }

    fn read_usb(&self, endpoint: EndPoint, buffer: &[u8]) -> DeviceResult<()> {
        let mut untransferred = buffer.len() as i32;

        loop {
            let mut transferred = 0;
            let result = unsafe {
                ffi::libusb_bulk_transfer(
                    self.handle,
                    endpoint as u8,
                    buffer.as_ptr() as *mut u8,
                    untransferred as i32,
                    &mut transferred,
                    1000,
                )
            };

            if result != 0 {
                error!("USB read error: {}", result);
                return Err(DeviceError::ReadError(String::from("USB read error")));
            }

            if transferred == untransferred {
                break;
            }

            untransferred -= transferred;
        }

        Ok(())
    }

    fn write_usb(&self, endpoint: EndPoint, buffer: &[u8]) -> DeviceResult<()> {
        let mut untransferred = buffer.len() as i32;

        loop {
            let mut transferred = 0;
            let result = unsafe {
                ffi::libusb_bulk_transfer(
                    self.handle,
                    endpoint as u8,
                    buffer.as_ptr() as *mut u8,
                    untransferred as i32,
                    &mut transferred,
                    1000,
                )
            };

            if result != 0 {
                error!("USB write error: {}", result);
                return Err(DeviceError::WriteError(String::from("USB write error")));
            }

            if transferred == untransferred {
                break;
            }

            untransferred -= transferred;
        }

        Ok(())
    }

    fn sync_delay(&self) -> DeviceResult<()> {
        loop {
            let buffer = [0u8; 1];
            self.write_usb(EndPoint::EP4, &buffer)?;
            self.read_usb(EndPoint::EP8, &buffer)?;

            if buffer[0] != 0 {
                break;
            }
        }

        info!("Sync delay done");

        Ok(())
    }

    fn command_active(&self) -> DeviceResult<()> {
        self.sync_delay()?;
        let buffer = [0x01u8, 0x00u8];

        self.write_usb(EndPoint::EP4, &buffer)?;
        info!("Command active");
        Ok(())
    }

    fn encrypt_table_read(&mut self) -> DeviceResult<()> {
        self.sync_delay()?;

        let command = [0x01u8, 0x0fu8];
        self.write_usb(EndPoint::EP4, &command)?;

        unsafe {
            let u8buffer = std::slice::from_raw_parts_mut(
                self.encrypt_table.as_mut_ptr() as *mut u8,
                std::mem::size_of::<[u16; 32]>(),
            );
            self.read_usb(EndPoint::EP6, &u8buffer)?;
        }

        Ok(())
    }

    fn decoded_encrypt_table(&mut self) {
        decode_encrypt_table(&mut self.encrypt_table);
        self.encode_index.replace(0);
        self.decode_index.replace(0);
    }

    fn read_cfg(&mut self) -> DeviceResult<()> {
        let mut cfg = [0u16; 64];
        // Read Cfg Spacd
        {
            self.sync_delay()?;
            let command = [0x01u8, 0x01u8];
            self.write_usb(EndPoint::EP4, &command)?;

            unsafe {
                let u8buffer = std::slice::from_raw_parts_mut(
                    cfg.as_mut_ptr() as *mut u8,
                    std::mem::size_of::<[u16; 64]>(),
                );
                self.read_usb(EndPoint::EP6, &u8buffer)?;
            }

            self.command_active()?;
        }

        self.decrypt(&mut cfg);
        self.cfg = Cfg { cfg };

        info!("Successfully read Cfg Space");

        Ok(())
    }

    fn decrypt_base(&self, buffer: &mut [u16]) {
        let encript_key = &self.encrypt_table[16..32];
        let mut decode_index = *self.decode_index.borrow();

        for i in 0..buffer.len() {
            buffer[i] ^= encript_key[decode_index];
            decode_index = (decode_index + 1) & 0x0f;
        }

        self.decode_index.replace(decode_index);
    }

    fn decrypt<T>(&self, buffer: &mut [T]) {
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut u16,
                buffer.len() * std::mem::size_of::<T>() / 2,
            )
        };

        self.decrypt_base(buffer);
    }

    fn encrypt_base(&self, buffer: &mut [u16]) {
        let encript_key = &self.encrypt_table[0..16];
        let mut encode_index = *self.encode_index.borrow();

        for i in 0..buffer.len() {
            buffer[i] ^= encript_key[encode_index];
            encode_index = (encode_index + 1) & 0x0f;
        }

        self.encode_index.replace(encode_index);
    }

    fn encrypt<T>(&self, buffer: &mut [T]) {
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                buffer.as_mut_ptr() as *mut u16,
                buffer.len() * std::mem::size_of::<T>() / 2,
            )
        };

        self.encrypt_base(buffer);
    }

    pub fn init(&mut self) -> DeviceResult<()> {
        self.encrypt_table_read()?;
        self.decoded_encrypt_table();

        self.read_cfg()?;

        Ok(())
    }

    fn activate_fpga_programmer(&self) -> DeviceResult<()> {
        self.sync_delay()?;

        let command = [0x01u8, 0x02u8];
        self.write_usb(EndPoint::EP4, &command)?;

        info!("FPGA Programmer Activated");

        Ok(())
    }

    pub fn program(&mut self, bitfile: &std::path::Path) -> DeviceResult<()> {
        // Check if file is readable
        let file = std::fs::File::open(bitfile).map_err(|e| {
            error!("File open error: {}", e);
            DeviceError::OtherError(String::from("File open error"))
        })?;

        let lines = std::io::BufReader::new(file).lines();
        let mut program_data = Vec::new();

        for line in lines {
            let line = line.map_err(|e| {
                error!("File read error: {}", e);
                DeviceError::OtherError(String::from("File read error"))
            })?;

            let line = line.trim();
            if line.len() == 0 {
                continue;
            }

            let mut data = 0u32;

            for c in line.as_bytes().iter() {
                match *c {
                    b'_' => {
                        program_data.push(data);
                        data = 0;
                        continue;
                    }
                    b' ' | b'\t' => {
                        break;
                    }
                    _ => {}
                }

                let remapped = char_remap(c);
                if remapped.is_none() {
                    error!("Invalid character in bitfile");
                    return Err(DeviceError::OtherError(String::from(
                        "Invalid character in bitfile",
                    )));
                }

                data = (data << 4) | (remapped.unwrap() as u32);
            }
            program_data.push(data);
        }
        self.encrypt(&mut program_data);
        let u8program_data = unsafe {
            std::slice::from_raw_parts_mut(
                program_data.as_mut_ptr() as *mut u8,
                program_data.len() * std::mem::size_of::<u32>(),
            )
        };

        self.activate_fpga_programmer()?;

        let fifo_size = self.cfg.fifo_size() as usize;
        info!("Fifo size: {} * 16 bits", fifo_size);

        let max_single_transfer_size = fifo_size * 2;
        {
            let program_data_size = u8program_data.len();
            info!("Program data size: {} bytes", program_data_size);

            let mut offset = 0;
            
            while offset < program_data_size {
                let mut transfer_size = max_single_transfer_size;
                if offset + transfer_size > program_data_size {
                    transfer_size = program_data_size - offset;
                }

                self.write_usb(EndPoint::EP2, &u8program_data[offset..offset + transfer_size])?;
                offset += transfer_size;
            }

            info!("Finished writing program data");
        }

        self.command_active()?;
        self.read_cfg()?;

        let programmed = self.cfg.is_programmed();
        if !programmed {
            error!("FPGA programming failed");
            return Err(DeviceError::OtherError(String::from(
                "FPGA programming failed",
            )));
        } else {
            info!("FPGA programming successful");
        }

        Ok(())
    }
}

fn decode_encrypt_table(encrypt_table: &mut [u16]) {
    encrypt_table[0] = !encrypt_table[0];

    for i in 1..encrypt_table.len() {
        encrypt_table[i] = encrypt_table[i] ^ encrypt_table[i - 1];
    }
}

fn char_remap(c: &u8) -> Option<u8> {
    let result = match c {
        0x30..=0x39 => c - 0x30,
        0x41..=0x46 => c - 0x37,
        0x61..=0x66 => c - 0x57,
        _ => return None,
    };

    Some(result)
}
