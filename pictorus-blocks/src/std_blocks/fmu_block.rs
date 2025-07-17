use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use fmu_runner::{fmi2Type, model_description::ScalarVariable, Fmu, FmuInstance, FmuLibrary};
use pictorus_block_data::BlockData as OldBlockData;
use pictorus_traits::{Context, ProcessBlock};
use std::collections::HashMap;

/// The FMU block is a wrapper around an FMU file that allows it to be used as a block in a simulation.
/// It takes a set of parameters that define the FMU file, the input signals, and the output signals.
/// Each time step, it will run the FMU for the given time step with the provided inputs and return the output signals.
pub struct FmuBlock<const N_IN: usize, const N_OUT: usize> {
    pub data: Vec<OldBlockData>,
    fm_cs: Option<FmuInstance<FmuLibrary>>,
    buffer: [f64; N_OUT],
}

impl<const N_IN: usize, const N_OUT: usize> Default for FmuBlock<N_IN, N_OUT> {
    fn default() -> Self {
        Self {
            data: vec![OldBlockData::from_scalar(0.0); N_OUT],
            fm_cs: None,
            buffer: [0.0; N_OUT],
        }
    }
}

impl<const N_IN: usize, const N_OUT: usize> FmuBlock<N_IN, N_OUT> {
    fn run_time_step(
        &mut self,
        params: &Parameters,
        context: &dyn Context,
        inputs: &[f64; N_IN],
    ) -> [f64; N_OUT] {
        let fmu = self.fm_cs.get_or_insert_with(|| {
            Self::build_fmu(params).expect("Failed to load and instantiate FMU")
        });

        let signals = fmu.lib.variables();
        // params.input_signals should give us the names of the input signals
        // we can use those to index into the signals map and then set the values
        let mapped_inputs: HashMap<&ScalarVariable, f64> = params
            .input_signals
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let signal = signals.get(name).expect("Signal not found in FMU");
                let input = inputs
                    .get(i)
                    .expect("Size mismatch between provided inputs and expected inputs");
                (signal, *input)
            })
            .collect();
        fmu.set_reals(&mapped_inputs)
            .expect("Failed to set FMU inputs");

        // run the FMU for the time step
        if let Some(curr_timestep) = context.timestep() {
            let step_start_time = context.time() - curr_timestep;
            fmu.do_step(
                step_start_time.as_secs_f64(),
                curr_timestep.as_secs_f64(),
                false,
            )
            .expect("Failed to do FMU step");
        }

        // Build the return value
        let mut output_data = [0.0; N_OUT];
        if N_OUT == 0 {
            // Special case for no outputs
            return output_data;
        }

        // Get the signals we care about (in return order)
        let desired_outputs = params
            .output_signals
            .iter()
            .map(|name| signals.get(name).expect("Signal not found in FMU"))
            .collect::<Vec<_>>();
        // Get the values from the FMU
        let model_outputs = fmu
            .get_reals(&desired_outputs)
            .expect("Failed to get FMU outputs");
        // Copy the fmu outputs to the output data
        for (signal, output_value) in desired_outputs
            .iter()
            .map(|s| model_outputs.get(s).expect("Failed to get FMU output"))
            .zip(output_data.iter_mut())
        {
            *output_value = *signal;
        }
        output_data
    }

    fn build_fmu(params: &Parameters) -> Result<FmuInstance<FmuLibrary>, FmuErrors> {
        let fmu = Fmu::unpack(&params.fmu_path)?.load(fmi2Type::fmi2CoSimulation)?;
        let fmu_cs = FmuInstance::instantiate(fmu, false)?;
        let signals = fmu_cs.lib.variables();
        fmu_cs.setup_experiment(0.0, None, None)?;
        fmu_cs.enter_initialization_mode()?;
        let param_values = params
            .fmu_params
            .iter()
            .map(|(k, v)| (&signals[k], *v))
            .collect::<HashMap<_, _>>();
        fmu_cs.set_reals(&param_values)?;
        fmu_cs.exit_initialization_mode()?;

        Ok(fmu_cs)
    }
}

/// This allows us to return errors from the FMU library from `build_fmu`. This is mainly used to allow
/// `?` operator early returns, but down the line could be useful if we flush out how we handle fallibility
#[derive(Debug)]
#[allow(dead_code)]
enum FmuErrors {
    Fmu(fmu_runner::FmuError),
    FmuLoad(fmu_runner::FmuLoadError),
    FmuUnpack(fmu_runner::FmuUnpackError),
}

impl From<fmu_runner::FmuError> for FmuErrors {
    fn from(err: fmu_runner::FmuError) -> Self {
        FmuErrors::Fmu(err)
    }
}

impl From<fmu_runner::FmuLoadError> for FmuErrors {
    fn from(err: fmu_runner::FmuLoadError) -> Self {
        FmuErrors::FmuLoad(err)
    }
}

impl From<fmu_runner::FmuUnpackError> for FmuErrors {
    fn from(err: fmu_runner::FmuUnpackError) -> Self {
        FmuErrors::FmuUnpack(err)
    }
}

impl<const N_IN: usize, const N_OUT: usize> ProcessBlock for FmuBlock<N_IN, N_OUT> {
    type Parameters = Parameters;
    // We use homogeneous arrays for inputs and outputs to avoid the limits/complexity
    // of mixed data types. This is safe to do because we only support FMI 2.0 which
    // only supports scalar values. If we add support for FMI 3.0 we will need to revisit this.
    type Inputs = [f64; N_IN];
    type Output = [f64; N_OUT];

    #[allow(unused_variables)]
    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        context: &dyn Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        self.buffer = self.run_time_step(parameters, context, inputs);
        self.data.clear();
        self.data = self
            .buffer
            .iter()
            .map(|&x| OldBlockData::from_scalar(x))
            .collect();
        &self.buffer
    }
}

/// Parameters for the FMU block.
pub struct Parameters {
    /// The path to the FMU file.
    pub fmu_path: String,
    /// The parameters for the FMU.
    /// Note that these are set on the first execution of the FMU block.
    /// and subsequent calls to the block will not change them.
    /// This is a HashMap of parameter name to value.
    pub fmu_params: HashMap<String, f64>,
    /// The input signals for the FMU, oder is important and defines the order the block expects the inputs to be in
    pub input_signals: Vec<String>,
    /// The output signals for the FMU, order is important and defines the order the block sets the outputs to be in
    pub output_signals: Vec<String>,
}

impl Parameters {
    pub fn new(
        fmu_path: &str,
        fmu_params: &HashMap<&'static str, f64>,
        input_signals: Vec<String>,
        output_signals: Vec<String>,
    ) -> Self {
        Self {
            fmu_path: fmu_path.to_string(),
            fmu_params: fmu_params
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
            input_signals,
            output_signals,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_impls_associated_types() {
        // Cover edge cases and the start of the range
        let _: <FmuBlock<0, 0> as pictorus_traits::ProcessBlock>::Inputs = [];
        let _: <FmuBlock<0, 0> as pictorus_traits::ProcessBlock>::Output = [];
        let _: <FmuBlock<1, 0> as pictorus_traits::ProcessBlock>::Inputs = [0.0];
        let _: <FmuBlock<1, 0> as pictorus_traits::ProcessBlock>::Output = [];
        let _: <FmuBlock<1, 1> as pictorus_traits::ProcessBlock>::Inputs = [0.0];
        let _: <FmuBlock<1, 1> as pictorus_traits::ProcessBlock>::Output = [0.0];
        let _: <FmuBlock<2, 0> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0];
        let _: <FmuBlock<2, 0> as pictorus_traits::ProcessBlock>::Output = [];
        let _: <FmuBlock<2, 1> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0];
        let _: <FmuBlock<2, 1> as pictorus_traits::ProcessBlock>::Output = [0.0];
        let _: <FmuBlock<2, 2> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0];
        let _: <FmuBlock<2, 2> as pictorus_traits::ProcessBlock>::Output = [0.0, 1.0];
        let _: <FmuBlock<2, 3> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0];
        let _: <FmuBlock<2, 3> as pictorus_traits::ProcessBlock>::Output = [0.0, 1.0, 2.0];
        let _inputs: <FmuBlock<3, 0> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0, 2.0];
        let _output: <FmuBlock<3, 0> as pictorus_traits::ProcessBlock>::Output = [];
        let _inputs: <FmuBlock<2, 3> as pictorus_traits::ProcessBlock>::Inputs = [1.0, 2.0];
        let _output: <FmuBlock<2, 3> as pictorus_traits::ProcessBlock>::Output = [3.0, 4.0, 5.0];

        // cover a smattering of random cases
        let _: <FmuBlock<3, 4> as pictorus_traits::ProcessBlock>::Inputs = [0.0, 1.0, 2.0];
        let _: <FmuBlock<3, 4> as pictorus_traits::ProcessBlock>::Output = [0.0, 1.0, 2.0, 3.0];
        let _: <FmuBlock<8, 8> as pictorus_traits::ProcessBlock>::Inputs =
            [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let _: <FmuBlock<8, 8> as pictorus_traits::ProcessBlock>::Output =
            [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let _: <FmuBlock<7, 2> as pictorus_traits::ProcessBlock>::Inputs =
            [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let _: <FmuBlock<7, 2> as pictorus_traits::ProcessBlock>::Output = [0.0, 1.0];
    }
}
