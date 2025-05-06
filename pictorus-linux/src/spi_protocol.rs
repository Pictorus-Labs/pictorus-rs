use std::io::{Read, Write};

use linux_embedded_hal::spidev::{Spidev, SpidevOptions};
use pictorus_blocks::{SpiReceiveBlockParams, SpiTransmitBlockParams};
use pictorus_traits::ByteSliceSignal;
use pictorus_traits::{Context, InputBlock, OutputBlock, PassBy};

use super::CdevPin;
use pictorus_internal::protocols::{Flush, OutputPin};
use pictorus_internal::utils::PictorusError;

pub struct SpiConnection {
    device: Spidev,
    cs: CdevPin,
    cache: Vec<u8>,
    is_cache_valid: bool,
}

impl SpiConnection {
    pub fn new(
        port: &'static str,
        frequency: u32,
        bits_per_transfer: u8,
        lsb_first: bool,
        mode: &'static str,
        cs: CdevPin,
    ) -> Result<Self, PictorusError> {
        let mut spi = Spidev::open(port).map_err(|_err| {
            PictorusError::new("SpiConnection".into(), "Failed to open SPI device".into())
        })?;

        let mut options = SpidevOptions::new();
        match mode {
            "1" => options.mode(linux_embedded_hal::spidev::SpiModeFlags::SPI_MODE_1),
            "2" => options.mode(linux_embedded_hal::spidev::SpiModeFlags::SPI_MODE_2),
            "3" => options.mode(linux_embedded_hal::spidev::SpiModeFlags::SPI_MODE_3),
            _ => options.mode(linux_embedded_hal::spidev::SpiModeFlags::SPI_MODE_0),
        };

        options
            .bits_per_word(bits_per_transfer)
            .max_speed_hz(frequency)
            .lsb_first(lsb_first);

        spi.configure(&options).map_err(|_err| {
            PictorusError::new(
                "SpiConnection".into(),
                "Failed to configure SPI device".into(),
            )
        })?;

        Ok(SpiConnection {
            device: spi,
            cs,
            cache: Vec::new(),
            is_cache_valid: false,
        })
    }
}

impl InputBlock for SpiConnection {
    type Output = ByteSliceSignal;
    type Parameters = SpiReceiveBlockParams;

    fn input(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        if !self.is_cache_valid {
            self.is_cache_valid = true;

            // Resize cache
            self.cache.resize(parameters.read_bytes, 0);

            // Attempt to read
            let result = self
                .device
                .read_exact(self.cache.as_mut_slice())
                .map_err(|_err| {
                    PictorusError::new(
                        "SpiConnection".into(),
                        "Failed to read from SPI device in ::read".into(),
                    )
                });

            if result.is_err() {
                // TODO: Error handling?
                // Keep the results, good or bad, in memory
            }

            let result = self.cs.set_high().map_err(|_err| {
                PictorusError::new(
                    "SpiConnection".into(),
                    "Failed to set CS pin in ::write".into(),
                )
            });

            if result.is_err() {
                // TODO: Error handling?
                // Keep the results, good or bad, in memory
            }
        }

        &self.cache
    }
}

impl OutputBlock for SpiConnection {
    type Inputs = ByteSliceSignal;
    type Parameters = SpiTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        // TODO: Error handling?
        self.cs
            .set_low()
            .map_err(|_err| {
                PictorusError::new(
                    "SpiConnection".into(),
                    "Failed to set CS pin in ::write".into(),
                )
            })
            .ok();

        // TODO: Error handling?
        self.device
            .write(inputs)
            .map_err(|_err| {
                PictorusError::new(
                    "SpiConnection".into(),
                    "Failed to write to SPI device in ::write_u8".into(),
                )
            })
            .ok();
    }
}

impl Flush for SpiConnection {
    fn flush(&mut self) {
        // Automatically set CS high after flush
        self.cs
            .set_high()
            .map_err(|_err| {
                PictorusError::new(
                    "SpiConnection".into(),
                    "Failed to set CS pin in ::write".into(),
                )
            })
            .ok();
        self.cache.clear();
        self.is_cache_valid = false;
    }
}
