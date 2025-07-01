use std::io;

use embedded_io::{ErrorType, Read, Write};
use log::{debug, info};
use pictorus_blocks::{SerialReceiveBlockParams, SerialTransmitBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock};
use serialport::{self, SerialPort};

use pictorus_internal::protocols::BUFF_SIZE_BYTES;
use pictorus_internal::utils::PictorusError;

pub fn create_serial_port(
    port: &str,
    baud_rate: f64,
    transmit_enabled: bool,
) -> Result<Option<Box<dyn SerialPort>>, PictorusError> {
    if !transmit_enabled {
        return Ok(None);
    }

    let baud_rate = baud_rate as u32;
    let port = serialport::new(port, baud_rate).open().map_err(|err| {
                let message = match err.kind() {
                    serialport::ErrorKind::NoDevice => {
                        format!("Failed to bind to serial port: {port} - This could indicate that the device is in use by another process or was disconnected while performing I/O.")
                    }
                    serialport::ErrorKind::Io(_) => {
                        format!("Failed to bind to serial port: {port} - Does it exist?")
                    }
                    _ => format!("Unknown error! Unable to connect to serial port: {port} ({err})"),
                };
                PictorusError::new("SerialProtocol".into(), message)
            })?;
    Ok(Some(port))
}

pub struct SerialConnection {
    port: Option<Box<dyn SerialPort>>,
    cache: Vec<u8>,
    is_cache_valid: bool,
    port_addr: String,
}

impl SerialConnection {
    pub fn new(port: &str, baud: f64, transmit_enabled: bool) -> Result<Self, PictorusError> {
        info!("Opening serial port {port} with baud {baud}");
        Ok(SerialConnection {
            port: create_serial_port(port, baud, transmit_enabled)?,
            cache: Vec::new(),
            port_addr: port.to_string(),
            is_cache_valid: false,
        })
    }
}

impl InputBlock for SerialConnection {
    type Output = ByteSliceSignal;
    type Parameters = SerialReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        if let Ok(len) = self.read(&mut []) {
            self.cache.resize(len, 0);
        }
        &self.cache
    }
}

impl ErrorType for SerialConnection {
    type Error = io::Error;
}

impl Read for SerialConnection {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, io::Error> {
        if !self.is_cache_valid {
            if let Some(port) = &mut self.port {
                self.cache.resize(BUFF_SIZE_BYTES, 0);
                let res = port.read(&mut self.cache);
                let size = match res {
                    Ok(size) => size,
                    Err(err) => {
                        // TODO: Handle error
                        // Keep the results, good or bad, in memory
                        return Err(err);
                    }
                };
                self.is_cache_valid = true;
                return Ok(size);
            }
        } else {
            let len = self.cache.len();
            if len == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "No data available",
                ));
            }
            return Ok(len);
        }
        Err(io::Error::new(io::ErrorKind::NotConnected, "I/O disabled"))
    }
}

impl Write for SerialConnection {
    fn write(&mut self, buf: &[u8]) -> Result<usize, io::Error> {
        if let Some(port) = &mut self.port {
            return port.write(buf);
        }
        Err(io::Error::new(io::ErrorKind::NotConnected, "I/O disabled"))
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        self.cache.clear();
        self.is_cache_valid = false;
        Ok(())
    }
}

impl pictorus_traits::OutputBlock for SerialConnection {
    type Inputs = pictorus_traits::ByteSliceSignal;
    type Parameters = SerialTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        self.write(inputs).ok();
    }
}

impl Drop for SerialConnection {
    fn drop(&mut self) {
        debug!("Closing serial port {0}", self.port_addr);
        if let Some(port) = &mut self.port {
            let _ = port.flush();
        }
    }
}
