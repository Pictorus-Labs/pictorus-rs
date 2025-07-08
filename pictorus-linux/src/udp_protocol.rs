use std::io::{Error, ErrorKind};
use std::net::UdpSocket;

use log::debug;
use pictorus_blocks::{UdpReceiveBlockParams, UdpTransmitBlockParams};
use pictorus_traits::{ByteSliceSignal, InputBlock, OutputBlock};

use pictorus_internal::protocols::{BUFF_SIZE_BYTES, UdpProtocol};
use pictorus_internal::utils::PictorusError;

const ERR_TYPE: &str = "UdpProtocol";

fn create_udp_socket(
    address: &str,
    transmit_enabled: bool,
) -> Result<Option<UdpSocket>, PictorusError> {
    if !transmit_enabled {
        return Ok(None);
    }

    let socket = UdpSocket::bind(address).map_err(|err| {
        let message = match err.kind() {
            ErrorKind::InvalidInput => format!("Couldn't bind UDP receiver at invalid address: {address} - Is this address valid?"),
            ErrorKind::AddrInUse => format!("Couldn't bind UDP receiver at already bound address: {address} - Is another process currently bound here?"),
            _ => format!("Unknown error! Couldn't bind UDP receiver at address: {address} ({err})"),
        };
        PictorusError::new(
            ERR_TYPE.into(),
            message,
        )
    })?;

    socket.set_nonblocking(true).map_err(|_| {
        PictorusError::new(
            ERR_TYPE.into(),
            format!("Failed to set nonblocking on UDP port at address: {address}",),
        )
    })?;
    Ok(Some(socket))
}

pub struct UdpConnection {
    socket: Option<UdpSocket>,
    cache: Option<Vec<u8>>,
}

impl UdpConnection {
    pub fn new(address: &str, transmit_enabled: bool) -> Result<Self, PictorusError> {
        Ok(UdpConnection {
            cache: None,
            socket: create_udp_socket(address, transmit_enabled)?,
        })
    }

    fn read_into_vec(&mut self) -> Result<Vec<u8>, Error> {
        if let Some(socket) = &mut self.socket {
            // Set the cache regardless of the result so we don't read again until flush is called
            let mut num_bytes_read = 0;
            let mut output = vec![0; BUFF_SIZE_BYTES];
            // Only use the most recent value on the socket
            while let Ok((n, _)) = socket.recv_from(&mut output) {
                num_bytes_read = n;
            }

            debug!("Received {num_bytes_read} bytes");
            output.resize(num_bytes_read, 0);
            Ok(output)
        } else {
            Err(Error::new(ErrorKind::NotConnected, "I/O disabled"))
        }
    }
}

impl UdpProtocol for UdpConnection {
    fn read(&mut self) -> Result<&[u8], Error> {
        let cache = match self.cache {
            Some(ref mut cache) => cache,
            None => {
                let read_bytes = self.read_into_vec()?;
                self.cache.insert(read_bytes)
            }
        };

        if cache.is_empty() {
            Err(Error::new(ErrorKind::WouldBlock, "No data received"))
        } else {
            Ok(cache.as_slice())
        }
    }

    fn write(&mut self, buf: &[u8], to_addr: &str) -> Result<usize, Error> {
        if let Some(socket) = &mut self.socket {
            return socket.send_to(buf, to_addr);
        }

        Ok(0)
    }

    fn flush(&mut self) {
        self.cache = None;
    }
}

impl InputBlock for UdpConnection {
    type Output = ByteSliceSignal;
    type Parameters = UdpReceiveBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
    ) -> pictorus_traits::PassBy<'_, Self::Output> {
        self.read().unwrap_or_default()
    }
}

impl OutputBlock for UdpConnection {
    type Inputs = ByteSliceSignal;
    type Parameters = UdpTransmitBlockParams;

    fn output(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        self.write(inputs, parameters.destination()).ok();
    }
}
