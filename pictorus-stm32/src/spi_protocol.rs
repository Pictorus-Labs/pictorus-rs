use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi::Spi;
use heapless::Vec;
use log::warn;
use pictorus_blocks::{SpiReceiveBlockParams, SpiTransmitBlockParams};
use pictorus_internal::protocols::{BUFF_SIZE_BYTES, Flush};
use pictorus_traits::ByteSliceSignal;
use pictorus_traits::{Context, InputBlock, OutputBlock, PassBy};

pub struct SpiWrapper<'a> {
    spi: Spi<'a, Blocking>,
    bits_per_transfer: u8,
    cs: Output<'a>,
    cache: Vec<u8, BUFF_SIZE_BYTES>,
    cache_stale: bool,
}

impl<'a> SpiWrapper<'a> {
    pub fn new(spi: Spi<'a, Blocking>, bits_per_transfer: u8, cs_pin: Output<'a>) -> Self {
        Self {
            spi,
            bits_per_transfer,
            cs: cs_pin,
            cache: Vec::new(),
            cache_stale: true,
        }
    }
}

impl InputBlock for SpiWrapper<'_> {
    type Output = ByteSliceSignal;
    type Parameters = SpiReceiveBlockParams;

    fn input<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'b, Self::Output> {
        if self.cache_stale {
            self.cache_stale = false;

            if parameters.read_bytes != 0 {
                self.cache.resize(parameters.read_bytes, 0).ok();
                let retval = self.spi.blocking_read(self.cache.as_mut_slice());
                if retval.is_err() {
                    // TODO: Error handling?
                    // Keep the results, good or bad, in memory
                }
            }
        }

        self.cs.set_high();

        &self.cache
    }
}

impl OutputBlock for SpiWrapper<'_> {
    type Inputs = ByteSliceSignal;
    type Parameters = SpiTransmitBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) {
        self.cs.set_low();
        let result = match self.bits_per_transfer {
            1..=8 => self.spi.blocking_write(inputs),
            9..=16 => {
                if inputs.len() % 2 != 0 {
                    warn!("Data length is not a multiple of 2, dropping last byte");
                }

                // TODO: Error handling?
                inputs.chunks_exact(2).try_for_each(|chunk| {
                    let mut val = [0u16; 1];
                    val[0] = u16::from_le_bytes([chunk[1], chunk[0]]);
                    self.spi.blocking_write(&[val[0]])
                })
            }
            _ => self.spi.blocking_write(inputs),
        };

        // TODO: Error handling
        if result.is_err() {
            warn!("SPI write error");
        }
    }
}

impl Flush for SpiWrapper<'_> {
    fn flush(&mut self) {
        self.cache_stale = true;
        // Automatically set CS high after flush
        self.cs.set_high();
        self.cache.clear();
    }
}
