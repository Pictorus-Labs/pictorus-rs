use embassy_stm32::dac::Dac;
use pictorus_blocks::DacBlockParams;
use pictorus_traits::{Matrix, OutputBlock};

pub struct DacWrapper<
    'a,
    T: embassy_stm32::dac::Instance,
    const CHANNELS: usize,
    const SAMPLES: usize,
> {
    dac: Dac<'a, T>,
}

impl<'a, T, const CHANNELS: usize, const SAMPLES: usize> DacWrapper<'a, T, CHANNELS, SAMPLES>
where
    T: embassy_stm32::dac::Instance,
{
    pub fn new(dac: Dac<'a, T>) -> Self {
        Self { dac }
    }

    pub fn configure(&mut self) {
        // Note: A lot of the configuration options disable the DAC
        self.dac
            .ch1()
            .set_trigger(embassy_stm32::dac::TriggerSel::Software);
        self.dac
            .ch2()
            .set_trigger(embassy_stm32::dac::TriggerSel::Software);

        self.dac.ch1().set_triggering(true);
        self.dac.ch2().set_triggering(true);

        // Re-enable the DAC after making all the settings adjustments
        self.dac.ch1().enable();
        self.dac.ch2().enable();
    }
}

impl<const CHANNELS: usize, const SAMPLES: usize, T> OutputBlock
    for DacWrapper<'_, T, CHANNELS, SAMPLES>
where
    T: embassy_stm32::dac::Instance,
{
    type Inputs = Matrix<SAMPLES, CHANNELS, f64>;
    type Parameters = DacBlockParams;

    fn output(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) {
        self.dac.ch1().set(embassy_stm32::dac::Value::Bit12Right(
            inputs.data[0][0] as u16,
        ));
        self.dac.ch2().set(embassy_stm32::dac::Value::Bit12Right(
            inputs.data[1][0] as u16,
        ));
        self.dac.ch1().trigger();
        self.dac.ch2().trigger();
    }
}
