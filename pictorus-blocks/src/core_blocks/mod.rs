// Add new core blocks here
mod abs_block;
pub use abs_block::AbsBlock;

mod adc_block;
pub use adc_block::AdcBlock;
#[doc(hidden)]
pub use adc_block::Parameters as AdcBlockParams;

mod aggregate_block;
pub use aggregate_block::AggregateBlock;

mod app_time_block;
pub use app_time_block::AppTimeBlock;

mod arg_min_max_block;
pub use arg_min_max_block::ArgMinMaxBlock;

mod bias_block;
pub use bias_block::BiasBlock;

mod bit_shift_block;
pub use bit_shift_block::BitShiftBlock;

mod bitwise_operator_block;
pub use bitwise_operator_block::BitwiseOperatorBlock;

mod bytes_literal_block;
pub use bytes_literal_block::BytesLiteralBlock;

mod can_receive_block;
pub use can_receive_block::CanReceiveBlock;
#[doc(hidden)]
pub use can_receive_block::Parameters as CanReceiveBlockParams;

mod change_detection_block;
pub use change_detection_block::ChangeDetectionBlock;

mod clamp_block;
pub use clamp_block::ClampBlock;

mod comparison_block;
pub use comparison_block::ComparisonBlock;

mod compare_to_value_block;
pub use compare_to_value_block::CompareToValueBlock;

mod constant_block;
pub use constant_block::ConstantBlock;

mod counter_block;
pub use counter_block::CounterBlock;

mod cross_product_block;
pub use cross_product_block::CrossProductBlock;

mod dac_block;
pub use dac_block::DacBlock;
#[doc(hidden)]
pub use dac_block::Parameters as DacBlockParams;

mod deadband_block;
pub use deadband_block::DeadbandBlock;

mod delay_block;
pub use delay_block::DelayBlock;

mod delay_control_block;
pub use delay_control_block::DelayControlBlock;

mod determinant_block;
pub use determinant_block::DeterminantBlock;

mod derivative_block;
pub use derivative_block::DerivativeBlock;

mod dot_product_block;
pub use dot_product_block::DotProductBlock;

mod exponent_block;
pub use exponent_block::ExponentBlock;

// These blocks are special versions of passthrough blocks that are
// used to handle user-input functions that might return non-finite data
mod fix_non_finite_block;
/// Run a basic user-defined Rust function that returns a single value.
#[doc(inline)]
pub use fix_non_finite_block::FixNonFiniteBlock as RustCodeBlock;
/// Run a user-defined equation that returns a single value.
#[doc(inline)]
pub use fix_non_finite_block::FixNonFiniteBlock as EquationBlock;

mod frequency_filter_block;
pub use frequency_filter_block::FrequencyFilterBlock;

mod gain_block;
pub use gain_block::GainBlock;

mod gpio_output_block;
pub use gpio_output_block::GpioOutputBlock;
#[doc(hidden)]
pub use gpio_output_block::Parameters as GpioOutputBlockParams;

mod iir_filter_block;
pub use iir_filter_block::IirFilterBlock;

mod integral_block;
pub use integral_block::IntegralBlock;

mod logical_block;
pub use logical_block::LogicalBlock;

mod lookup_2d_block;
pub use lookup_2d_block::Lookup2DBlock;

mod lookup_1d_block;
pub use lookup_1d_block::Lookup1DBlock;

mod min_max_block;
pub use min_max_block::MinMaxBlock;

mod matrix_inverse_block;
pub use matrix_inverse_block::{Inverse, MatrixInverseBlock, Svd};

mod noop_input_block;
pub use noop_input_block::NoOpInputBlock;

mod noop_output_block;
pub use noop_output_block::NoOpOutputBlock;

mod not_block;
pub use not_block::NotBlock;

// There are several blocks that just compute a value external to the block
// and pass it through.
mod passthrough_block;
#[doc(hidden)]
pub use passthrough_block::Parameters as GpioInputBlockParams;
#[doc(hidden)]
pub use passthrough_block::Parameters as SpiTransmitBlockParams;

/// Used to signify an input port of a component.
#[doc(inline)]
pub use passthrough_block::PassthroughBlock as ComponentInputBlock;
/// Reads data from a data store or variable and outputs it as a signal.
#[doc(inline)]
pub use passthrough_block::PassthroughBlock as DataReadBlock;
/// Stores the data from a GPIO input pin and outputs it as a signal.
#[doc(inline)]
pub use passthrough_block::PassthroughBlock as GpioInputBlock;
/// Stores data to be sent over SPI and outputs it as a signal.
#[doc(inline)]
pub use passthrough_block::PassthroughBlock as SpiTransmitBlock;

mod pid_block;
pub use pid_block::PidBlock;

mod product_block;
pub use product_block::{ComponentWise, MatrixMultiply, ProductBlock};

mod pwm_block;
#[doc(hidden)]
pub use pwm_block::Parameters as PwmBlockParams;
pub use pwm_block::PwmBlock;

mod quantize_block;
pub use quantize_block::QuantizeBlock;

mod ramp_block;
pub use ramp_block::RampBlock;

mod random_number_block;
pub use random_number_block::RandomNumberBlock;

mod rate_limit_block;
pub use rate_limit_block::RateLimitBlock;

mod sawtoothwave_block;
pub use sawtoothwave_block::SawtoothwaveBlock;

mod sinewave_block;
pub use sinewave_block::SinewaveBlock;

mod sliding_window_block;
#[doc(hidden)]
pub use sliding_window_block::Parameters as SlidingWindowBlockParams;
pub use sliding_window_block::SlidingWindowBlock;

mod squarewave_block;
pub use squarewave_block::SquarewaveBlock;

mod sum_block;
pub use sum_block::SumBlock;

mod timer_block;
pub use timer_block::TimerBlock;

mod transpose_block;
pub use transpose_block::TransposeBlock;

mod transfer_function_block;
pub use transfer_function_block::TransferFunctionBlock;

mod trianglewave_block;
pub use trianglewave_block::TrianglewaveBlock;

mod trigonometry_block;
pub use trigonometry_block::TrigonometryBlock;

mod vector_index_block;
pub use vector_index_block::VectorIndexBlock;

mod vector_merge_block;
pub use vector_merge_block::VectorMergeBlock;

mod vector_norm_block;
pub use vector_norm_block::VectorNormBlock;

mod vector_reshape_block;
pub use vector_reshape_block::VectorReshapeBlock;

mod vector_slice_block;
pub use vector_slice_block::VectorSliceBlock;

mod vector_sort_block;
pub use vector_sort_block::VectorSortBlock;
