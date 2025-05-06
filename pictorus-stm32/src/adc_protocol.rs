use embassy_stm32::adc::{Adc, AnyAdcChannel};
use pictorus_blocks::AdcBlockParams;
use pictorus_internal::protocols::Flush;
use pictorus_traits::{Context, InputBlock, PassBy};

pub struct AdcWrapper<'a, T: embassy_stm32::adc::Instance> {
    adc: Adc<'a, T>,
    channel: AnyAdcChannel<T>,
    buffer: Option<u16>,
}

impl<T> InputBlock for AdcWrapper<'_, T>
where
    T: embassy_stm32::adc::Instance,
{
    type Output = u16;
    type Parameters = AdcBlockParams;

    fn input(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn Context,
    ) -> PassBy<'_, Self::Output> {
        if self.buffer.is_none() {
            self.buffer = Some(self.adc.read(&mut self.channel));
        }

        self.buffer.unwrap_or(0)
    }
}

impl<T> Flush for AdcWrapper<'_, T>
where
    T: embassy_stm32::adc::Instance,
{
    fn flush(&mut self) {
        self.buffer = None;
    }
}

impl<'a, T> AdcWrapper<'a, T>
where
    T: embassy_stm32::adc::Instance,
{
    pub fn new(adc: Adc<'a, T>, channel: AnyAdcChannel<T>) -> Self {
        Self {
            adc,
            channel,
            buffer: None,
        }
    }
}
