//! Translated from https://www.app.pictor.us/app/68ae03a7aad158fcb5c48245
#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use alloc::{string::String, vec::Vec};
use core::ptr::addr_of_mut;
use core::time::Duration;
#[cfg(target_arch = "arm")]
use panic_halt as _;
use pictorus_block_data::{BlockData, ToPass};
use pictorus_blocks::{
    ComponentInputBlock, ComponentOutputBlock, ComponentWise, ConstantBlock, DeadbandBlock,
    ExponentBlock, GainBlock, MatrixMultiply, PidBlock, ProductBlock, RustCodeBlock, SumBlock,
    TransposeBlock, VectorIndexBlock, VectorMergeBlock,
};
use pictorus_internal::utils::PictorusError;
use pictorus_px4::message_impls::ActuatorArmed;
use pictorus_px4::{
    ffi_protocol::{FfiInputBlock, FfiOutputBlock, FfiProtocol},
    message_impls::{ActuatorMotors, ManualControlInput, VehicleOdometry},
};
use pictorus_traits::{
    Context as CorelibContext, GeneratorBlock, InputBlock, Matrix, OutputBlock, ProcessBlock,
};

#[global_allocator]
static HEAP: embedded_alloc::Heap = embedded_alloc::Heap::empty();

pub fn compile_info() -> &'static str {
    return "quad_attitude_68ae03a7aad158fcb5c48245 version : compiled 08/26/2025 - 19:47:59";
}

#[derive(Debug, Clone)]
pub enum State {
    Main48246State,
}

pub struct Actuatormixere8f47Component {
    last_time_s: f64,
    mixer_matrix_e8f48_param: <ConstantBlock<Matrix<4, 4, f64>> as GeneratorBlock>::Parameters,
    mixer_matrix_e8f48: ConstantBlock<Matrix<4, 4, f64>>,
    tau_thrust_e8f49_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    tau_thrust_e8f49: ComponentInputBlock<f64>,
    tau_pitch_e8f4a_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    tau_pitch_e8f4a: ComponentInputBlock<f64>,
    tau_roll_e8f4b_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    tau_roll_e8f4b: ComponentInputBlock<f64>,
    tau_yaw_e8f4c_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    tau_yaw_e8f4c: ComponentInputBlock<f64>,
    vector_merge1_e8f4d_param: <VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)> as ProcessBlock>::Parameters,
    vector_merge1_e8f4d: VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)>,
    product1_e8f52_param: <ProductBlock<(Matrix<1, 4, f64>, Matrix<4, 4, f64>), MatrixMultiply> as ProcessBlock>::Parameters,
    product1_e8f52: ProductBlock<(Matrix<1, 4, f64>, Matrix<4, 4, f64>), MatrixMultiply>,
    constant21_e8f56_param: <ConstantBlock<f64> as GeneratorBlock>::Parameters,
    constant21_e8f56: ConstantBlock<f64>,
    sum1_e8f57_param: <SumBlock<(Matrix<1, 4, f64>, f64)> as ProcessBlock>::Parameters,
    sum1_e8f57: SumBlock<(Matrix<1, 4, f64>, f64)>,
    vector_index1_e8f5e_param: <VectorIndexBlock<4, f64, Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    vector_index1_e8f5e: VectorIndexBlock<4, f64, Matrix<1, 4, f64>>,
    motor_1_e8f5a_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    motor_1_e8f5a: ComponentOutputBlock<f64>,
    motor_2_e8f5b_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    motor_2_e8f5b: ComponentOutputBlock<f64>,
    motor_3_e8f5c_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    motor_3_e8f5c: ComponentOutputBlock<f64>,
    motor_4_e8f5d_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    motor_4_e8f5d: ComponentOutputBlock<f64>,
}

impl Actuatormixere8f47Component {
    pub fn new(_context: &Context) -> Self {
        let mixer_matrix_e8f48_value = BlockData::new(
            4,
            4,
            &[
                1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
                -1.0,
            ],
        );

        let mixer_matrix_e8f48_ic = mixer_matrix_e8f48_value.clone();

        // Mixer Matrix
        let mixer_matrix_e8f48_param =
            <ConstantBlock<Matrix<4, 4, f64>> as GeneratorBlock>::Parameters::new(
                mixer_matrix_e8f48_ic.to_pass(),
            );
        let mixer_matrix_e8f48 = ConstantBlock::default();

        // Tau Thrust
        let tau_thrust_e8f49_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_thrust_e8f49 = ComponentInputBlock::default();

        // Tau Pitch
        let tau_pitch_e8f4a_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_pitch_e8f4a = ComponentInputBlock::default();

        // Tau Roll
        let tau_roll_e8f4b_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_roll_e8f4b = ComponentInputBlock::default();

        // Tau Yaw
        let tau_yaw_e8f4c_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_yaw_e8f4c = ComponentInputBlock::default();

        // VectorMerge1
        let vector_merge1_e8f4d_param = <VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)> as ProcessBlock>::Parameters::new();
        let vector_merge1_e8f4d = VectorMergeBlock::default();

        // Product1
        let product1_e8f52_param = <ProductBlock<
            (Matrix<1, 4, f64>, Matrix<4, 4, f64>),
            MatrixMultiply,
        > as ProcessBlock>::Parameters::new();
        let product1_e8f52 = ProductBlock::default();

        let constant21_e8f56_value = -1.000000;

        let constant21_e8f56_ic = BlockData::from_element(1, 1, constant21_e8f56_value);

        // Constant21
        let constant21_e8f56_param =
            <ConstantBlock<f64> as GeneratorBlock>::Parameters::new(constant21_e8f56_ic.to_pass());
        let constant21_e8f56 = ConstantBlock::default();

        let sum1_e8f57_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // Sum1
        let sum1_e8f57_param =
            <SumBlock<(Matrix<1, 4, f64>, f64)> as ProcessBlock>::Parameters::new(
                sum1_e8f57_gains.to_pass(),
            );
        let sum1_e8f57 = SumBlock::default();

        let vector_index1_e8f5e_select_indexes = Vec::from([
            String::from("Scalar:0"),
            String::from("Scalar:1"),
            String::from("Scalar:2"),
            String::from("Scalar:3"),
        ]);

        // VectorIndex1
        let vector_index1_e8f5e_param =
            <VectorIndexBlock<4, f64, Matrix<1, 4, f64>> as ProcessBlock>::Parameters::new(
                &vector_index1_e8f5e_select_indexes,
            );
        let vector_index1_e8f5e = VectorIndexBlock::default();

        // Motor 1
        let motor_1_e8f5a_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let motor_1_e8f5a = ComponentOutputBlock::default();

        // Motor 2
        let motor_2_e8f5b_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let motor_2_e8f5b = ComponentOutputBlock::default();

        // Motor 3
        let motor_3_e8f5c_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let motor_3_e8f5c = ComponentOutputBlock::default();

        // Motor 4
        let motor_4_e8f5d_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let motor_4_e8f5d = ComponentOutputBlock::default();

        Actuatormixere8f47Component {
            last_time_s: -1.0,
            mixer_matrix_e8f48_param,
            mixer_matrix_e8f48,
            tau_thrust_e8f49_param,
            tau_thrust_e8f49,
            tau_pitch_e8f4a_param,
            tau_pitch_e8f4a,
            tau_roll_e8f4b_param,
            tau_roll_e8f4b,
            tau_yaw_e8f4c_param,
            tau_yaw_e8f4c,
            vector_merge1_e8f4d_param,
            vector_merge1_e8f4d,
            product1_e8f52_param,
            product1_e8f52,
            constant21_e8f56_param,
            constant21_e8f56,
            sum1_e8f57_param,
            sum1_e8f57,
            vector_index1_e8f5e_param,
            vector_index1_e8f5e,
            motor_1_e8f5a_param,
            motor_1_e8f5a,
            motor_2_e8f5b_param,
            motor_2_e8f5b,
            motor_3_e8f5c_param,
            motor_3_e8f5c,
            motor_4_e8f5d_param,
            motor_4_e8f5d,
        }
    }

    pub fn run(
        &mut self,
        context: &mut Context,
        tau_thrust_e8f49: f64,
        tau_pitch_e8f4a: f64,
        tau_roll_e8f4b: f64,
        tau_yaw_e8f4c: f64,
    ) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // Mixer Matrix
        let mixer_matrix_e8f48_0 = self
            .mixer_matrix_e8f48
            .generate(&self.mixer_matrix_e8f48_param, &runtime_ctx);

        // Tau Thrust
        let tau_thrust_e8f49_0 = self.tau_thrust_e8f49.process(
            &self.tau_thrust_e8f49_param,
            &runtime_ctx,
            tau_thrust_e8f49,
        );

        // Tau Pitch
        let tau_pitch_e8f4a_0 = self.tau_pitch_e8f4a.process(
            &self.tau_pitch_e8f4a_param,
            &runtime_ctx,
            tau_pitch_e8f4a,
        );

        // Tau Roll
        let tau_roll_e8f4b_0 =
            self.tau_roll_e8f4b
                .process(&self.tau_roll_e8f4b_param, &runtime_ctx, tau_roll_e8f4b);

        // Tau Yaw
        let tau_yaw_e8f4c_0 =
            self.tau_yaw_e8f4c
                .process(&self.tau_yaw_e8f4c_param, &runtime_ctx, tau_yaw_e8f4c);

        // VectorMerge1
        let vector_merge1_e8f4d_0 = self.vector_merge1_e8f4d.process(
            &self.vector_merge1_e8f4d_param,
            &runtime_ctx,
            (
                tau_thrust_e8f49_0,
                tau_pitch_e8f4a_0,
                tau_roll_e8f4b_0,
                tau_yaw_e8f4c_0,
            ),
        );

        // Product1
        let product1_e8f52_0 = self.product1_e8f52.process(
            &self.product1_e8f52_param,
            &runtime_ctx,
            (&vector_merge1_e8f4d_0, &mixer_matrix_e8f48_0),
        );

        // Constant21
        let constant21_e8f56_0 = self
            .constant21_e8f56
            .generate(&self.constant21_e8f56_param, &runtime_ctx);

        // Sum1
        let sum1_e8f57_0 = self.sum1_e8f57.process(
            &self.sum1_e8f57_param,
            &runtime_ctx,
            (&product1_e8f52_0, constant21_e8f56_0),
        );

        // VectorIndex1
        let (
            vector_index1_e8f5e_0,
            vector_index1_e8f5e_1,
            vector_index1_e8f5e_2,
            vector_index1_e8f5e_3,
        ) = self.vector_index1_e8f5e.process(
            &self.vector_index1_e8f5e_param,
            &runtime_ctx,
            &sum1_e8f57_0,
        );

        // Motor 1
        let _ = self.motor_1_e8f5a.process(
            &self.motor_1_e8f5a_param,
            &runtime_ctx,
            vector_index1_e8f5e_0,
        );

        // Motor 2
        let _ = self.motor_2_e8f5b.process(
            &self.motor_2_e8f5b_param,
            &runtime_ctx,
            vector_index1_e8f5e_1,
        );

        // Motor 3
        let _ = self.motor_3_e8f5c.process(
            &self.motor_3_e8f5c_param,
            &runtime_ctx,
            vector_index1_e8f5e_2,
        );

        // Motor 4
        let _ = self.motor_4_e8f5d.process(
            &self.motor_4_e8f5d_param,
            &runtime_ctx,
            vector_index1_e8f5e_3,
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Actuatoroutpute8ed1Component {
    last_time_s: f64,
    control_e8ed6_param: <ComponentInputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    control_e8ed6: ComponentInputBlock<Matrix<1, 4, f64>>,
    vector_merge_e8f84_param: <VectorMergeBlock<
        Matrix<1, 12, f64>,
        (Matrix<1, 4, f64>, Matrix<1, 8, f64>),
    > as ProcessBlock>::Parameters,
    vector_merge_e8f84:
        VectorMergeBlock<Matrix<1, 12, f64>, (Matrix<1, 4, f64>, Matrix<1, 8, f64>)>,
    vector_transpose_param: <TransposeBlock<Matrix<1, 12, f64>> as ProcessBlock>::Parameters,
    vector_transpose: TransposeBlock<Matrix<1, 12, f64>>,
    timestamp_e8ed2_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    timestamp_e8ed2: ComponentInputBlock<f64>,
    actuator_motors_output_param: <FfiOutputBlock<ActuatorMotors> as OutputBlock>::Parameters,
    actuator_motors_output_block: FfiOutputBlock<ActuatorMotors>,
}

impl Actuatoroutpute8ed1Component {
    pub fn new(_context: &Context) -> Self {
        // control
        let control_e8ed6_param =
            <ComponentInputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters::new();
        let control_e8ed6 = ComponentInputBlock::default();

        // VectorMerge
        let vector_merge_e8f84_param = <VectorMergeBlock<
            Matrix<1, 12, f64>,
            (Matrix<1, 4, f64>, Matrix<1, 8, f64>),
        > as ProcessBlock>::Parameters::new();
        let vector_merge_e8f84 = VectorMergeBlock::default();

        let vector_transpose_param =
            <TransposeBlock<Matrix<1, 12, f64>> as ProcessBlock>::Parameters::new();
        let vector_transpose = TransposeBlock::default();

        // Timestamp
        let timestamp_e8ed2_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let timestamp_e8ed2 = ComponentInputBlock::default();

        // GpioOutput3
        let actuator_motors_output_param =
            <FfiOutputBlock<ActuatorMotors> as OutputBlock>::Parameters::new();
        let actuator_motors_output_block = FfiOutputBlock::default();

        Actuatoroutpute8ed1Component {
            last_time_s: -1.0,
            control_e8ed6_param,
            control_e8ed6,
            vector_merge_e8f84_param,
            vector_merge_e8f84,
            vector_transpose_param,
            vector_transpose,
            timestamp_e8ed2_param,
            timestamp_e8ed2,
            actuator_motors_output_param,
            actuator_motors_output_block,
        }
    }

    pub fn run(
        &mut self,
        context: &mut Context,
        control_e8ed6: &Matrix<1, 4, f64>,
        timestamp_e8ed2: f64,
    ) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // control
        let control_e8ed6_0 =
            self.control_e8ed6
                .process(&self.control_e8ed6_param, &runtime_ctx, control_e8ed6);

        // VectorMerge
        let vector_merge_e8f84_0 = self.vector_merge_e8f84.process(
            &self.vector_merge_e8f84_param,
            &runtime_ctx,
            (&control_e8ed6_0, &Matrix::zeroed()),
        );

        // Vector Transpose
        let vector_transpose_0 = self.vector_transpose.process(
            &self.vector_transpose_param,
            &runtime_ctx,
            vector_merge_e8f84_0,
        );

        // Timestamp
        let timestamp_e8ed2_0 = self.timestamp_e8ed2.process(
            &self.timestamp_e8ed2_param,
            &runtime_ctx,
            timestamp_e8ed2,
        );

        // ActuatorMotors Output
        let _ = self.actuator_motors_output_block.output(
            &self.actuator_motors_output_param,
            &runtime_ctx,
            (timestamp_e8ed2_0, vector_transpose_0, 0.0),
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Attitudecontrollere8f24Component {
    last_time_s: f64,
    desired_yaw_rate_e8f27_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    desired_yaw_rate_e8f27: ComponentInputBlock<f64>,
    yaw_rate_e8f28_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    yaw_rate_e8f28: ComponentInputBlock<f64>,
    error_yaw_e8f34_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    error_yaw_e8f34: SumBlock<(f64, f64)>,
    pid3_e8f3c_param: <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters,
    pid3_e8f3c: PidBlock<f64, bool, 3>,
    tau_yaw_e8f2c_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    tau_yaw_e8f2c: ComponentOutputBlock<f64>,
    desired_pitch_e8f25_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    desired_pitch_e8f25: ComponentInputBlock<f64>,
    pitch_angle_e8f29_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    pitch_angle_e8f29: ComponentInputBlock<f64>,
    error_pitch_e8f30_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    error_pitch_e8f30: SumBlock<(f64, f64)>,
    pid1_e8f2f_param: <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters,
    pid1_e8f2f: PidBlock<f64, bool, 3>,
    tau_pitch_e8f2d_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    tau_pitch_e8f2d: ComponentOutputBlock<f64>,
    desired_roll_e8f26_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    desired_roll_e8f26: ComponentInputBlock<f64>,
    roll_angle_e8f2a_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    roll_angle_e8f2a: ComponentInputBlock<f64>,
    error_roll_e8f33_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    error_roll_e8f33: SumBlock<(f64, f64)>,
    pid2_e8f3b_param: <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters,
    pid2_e8f3b: PidBlock<f64, bool, 3>,
    tau_roll_e8f2e_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    tau_roll_e8f2e: ComponentOutputBlock<f64>,
}

impl Attitudecontrollere8f24Component {
    pub fn new(_context: &Context) -> Self {
        // Desired Yaw Rate
        let desired_yaw_rate_e8f27_param =
            <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let desired_yaw_rate_e8f27 = ComponentInputBlock::default();

        // Yaw Rate
        let yaw_rate_e8f28_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let yaw_rate_e8f28 = ComponentInputBlock::default();

        let error_yaw_e8f34_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Error Yaw
        let error_yaw_e8f34_param = <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(
            error_yaw_e8f34_gains.to_pass(),
        );
        let error_yaw_e8f34 = SumBlock::default();

        let pid3_e8f3c_kp = 1.000000;
        let pid3_e8f3c_ki = 0.000000;
        let pid3_e8f3c_kd = 0.000000;
        let pid3_e8f3c_i_max = 10000000000000000159028911097599180468360808563945281389781327557747838772170381060813469985856815104.000000;

        let pid3_e8f3c_ic = BlockData::new(1, 1, &[0.0]);

        // Pid3
        let pid3_e8f3c_param = <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters::new(
            pid3_e8f3c_ic.to_pass(),
            pid3_e8f3c_kp,
            pid3_e8f3c_ki,
            pid3_e8f3c_kd,
            pid3_e8f3c_i_max,
        );
        let pid3_e8f3c = PidBlock::default();

        // Tau Yaw
        let tau_yaw_e8f2c_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_yaw_e8f2c = ComponentOutputBlock::default();

        // Desired Pitch
        let desired_pitch_e8f25_param =
            <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let desired_pitch_e8f25 = ComponentInputBlock::default();

        // Pitch Angle
        let pitch_angle_e8f29_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let pitch_angle_e8f29 = ComponentInputBlock::default();

        let error_pitch_e8f30_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Error Pitch
        let error_pitch_e8f30_param = <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(
            error_pitch_e8f30_gains.to_pass(),
        );
        let error_pitch_e8f30 = SumBlock::default();

        let pid1_e8f2f_kp = 1.000000;
        let pid1_e8f2f_ki = 0.000000;
        let pid1_e8f2f_kd = 0.000000;
        let pid1_e8f2f_i_max = 10000000000000000159028911097599180468360808563945281389781327557747838772170381060813469985856815104.000000;

        let pid1_e8f2f_ic = BlockData::new(1, 1, &[0.0]);

        // Pid1
        let pid1_e8f2f_param = <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters::new(
            pid1_e8f2f_ic.to_pass(),
            pid1_e8f2f_kp,
            pid1_e8f2f_ki,
            pid1_e8f2f_kd,
            pid1_e8f2f_i_max,
        );
        let pid1_e8f2f = PidBlock::default();

        // Tau Pitch
        let tau_pitch_e8f2d_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_pitch_e8f2d = ComponentOutputBlock::default();

        // Desired Roll
        let desired_roll_e8f26_param =
            <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let desired_roll_e8f26 = ComponentInputBlock::default();

        // Roll Angle
        let roll_angle_e8f2a_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let roll_angle_e8f2a = ComponentInputBlock::default();

        let error_roll_e8f33_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Error Roll
        let error_roll_e8f33_param = <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(
            error_roll_e8f33_gains.to_pass(),
        );
        let error_roll_e8f33 = SumBlock::default();

        let pid2_e8f3b_kp = 1.000000;
        let pid2_e8f3b_ki = 0.000000;
        let pid2_e8f3b_kd = 0.000000;
        let pid2_e8f3b_i_max = 10000000000000000159028911097599180468360808563945281389781327557747838772170381060813469985856815104.000000;

        let pid2_e8f3b_ic = BlockData::new(1, 1, &[0.0]);

        // Pid2
        let pid2_e8f3b_param = <PidBlock<f64, bool, 3> as ProcessBlock>::Parameters::new(
            pid2_e8f3b_ic.to_pass(),
            pid2_e8f3b_kp,
            pid2_e8f3b_ki,
            pid2_e8f3b_kd,
            pid2_e8f3b_i_max,
        );
        let pid2_e8f3b = PidBlock::default();

        // Tau Roll
        let tau_roll_e8f2e_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let tau_roll_e8f2e = ComponentOutputBlock::default();

        Attitudecontrollere8f24Component {
            last_time_s: -1.0,
            desired_yaw_rate_e8f27_param,
            desired_yaw_rate_e8f27,
            yaw_rate_e8f28_param,
            yaw_rate_e8f28,
            error_yaw_e8f34_param,
            error_yaw_e8f34,
            pid3_e8f3c_param,
            pid3_e8f3c,
            tau_yaw_e8f2c_param,
            tau_yaw_e8f2c,
            desired_pitch_e8f25_param,
            desired_pitch_e8f25,
            pitch_angle_e8f29_param,
            pitch_angle_e8f29,
            error_pitch_e8f30_param,
            error_pitch_e8f30,
            pid1_e8f2f_param,
            pid1_e8f2f,
            tau_pitch_e8f2d_param,
            tau_pitch_e8f2d,
            desired_roll_e8f26_param,
            desired_roll_e8f26,
            roll_angle_e8f2a_param,
            roll_angle_e8f2a,
            error_roll_e8f33_param,
            error_roll_e8f33,
            pid2_e8f3b_param,
            pid2_e8f3b,
            tau_roll_e8f2e_param,
            tau_roll_e8f2e,
        }
    }

    pub fn run(
        &mut self,
        context: &mut Context,
        desired_pitch_e8f25: f64,
        desired_roll_e8f26: f64,
        desired_yaw_rate_e8f27: f64,
        pitch_angle_e8f29: f64,
        roll_angle_e8f2a: f64,
        yaw_rate_e8f28: f64,
    ) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // Desired Yaw Rate
        let desired_yaw_rate_e8f27_0 = self.desired_yaw_rate_e8f27.process(
            &self.desired_yaw_rate_e8f27_param,
            &runtime_ctx,
            desired_yaw_rate_e8f27,
        );

        // Yaw Rate
        let yaw_rate_e8f28_0 =
            self.yaw_rate_e8f28
                .process(&self.yaw_rate_e8f28_param, &runtime_ctx, yaw_rate_e8f28);

        // Error Yaw
        let error_yaw_e8f34_0 = self.error_yaw_e8f34.process(
            &self.error_yaw_e8f34_param,
            &runtime_ctx,
            (yaw_rate_e8f28_0, desired_yaw_rate_e8f27_0),
        );

        // Pid3
        let pid3_e8f3c_0 = self.pid3_e8f3c.process(
            &self.pid3_e8f3c_param,
            &runtime_ctx,
            (error_yaw_e8f34_0, false),
        );

        // Tau Yaw
        let _ = self
            .tau_yaw_e8f2c
            .process(&self.tau_yaw_e8f2c_param, &runtime_ctx, pid3_e8f3c_0);

        // Desired Pitch
        let desired_pitch_e8f25_0 = self.desired_pitch_e8f25.process(
            &self.desired_pitch_e8f25_param,
            &runtime_ctx,
            desired_pitch_e8f25,
        );

        // Pitch Angle
        let pitch_angle_e8f29_0 = self.pitch_angle_e8f29.process(
            &self.pitch_angle_e8f29_param,
            &runtime_ctx,
            pitch_angle_e8f29,
        );

        // Error Pitch
        let error_pitch_e8f30_0 = self.error_pitch_e8f30.process(
            &self.error_pitch_e8f30_param,
            &runtime_ctx,
            (desired_pitch_e8f25_0, pitch_angle_e8f29_0),
        );

        // Pid1
        let pid1_e8f2f_0 = self.pid1_e8f2f.process(
            &self.pid1_e8f2f_param,
            &runtime_ctx,
            (error_pitch_e8f30_0, false),
        );

        // Tau Pitch
        let _ =
            self.tau_pitch_e8f2d
                .process(&self.tau_pitch_e8f2d_param, &runtime_ctx, pid1_e8f2f_0);

        // Desired Roll
        let desired_roll_e8f26_0 = self.desired_roll_e8f26.process(
            &self.desired_roll_e8f26_param,
            &runtime_ctx,
            desired_roll_e8f26,
        );

        // Roll Angle
        let roll_angle_e8f2a_0 = self.roll_angle_e8f2a.process(
            &self.roll_angle_e8f2a_param,
            &runtime_ctx,
            roll_angle_e8f2a,
        );

        // Error Roll
        let error_roll_e8f33_0 = self.error_roll_e8f33.process(
            &self.error_roll_e8f33_param,
            &runtime_ctx,
            (desired_roll_e8f26_0, roll_angle_e8f2a_0),
        );

        // Pid2
        let pid2_e8f3b_0 = self.pid2_e8f3b.process(
            &self.pid2_e8f3b_param,
            &runtime_ctx,
            (error_roll_e8f33_0, false),
        );

        // Tau Roll
        let _ = self
            .tau_roll_e8f2e
            .process(&self.tau_roll_e8f2e_param, &runtime_ctx, pid2_e8f3b_0);

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Manualinpute8ec8Component {
    last_time_s: f64,
    time_stamp_e8ec9_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    time_stamp_e8ec9: ComponentOutputBlock<f64>,
    manual_input_param: <FfiInputBlock<ManualControlInput> as InputBlock>::Parameters,
    manual_input_block: FfiInputBlock<ManualControlInput>,
    vector_transpose_param: <TransposeBlock<Matrix<4, 1, f64>> as ProcessBlock>::Parameters,
    vector_transpose: TransposeBlock<Matrix<4, 1, f64>>,
    sticks_e8ecb_param: <ComponentOutputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    sticks_e8ecb: ComponentOutputBlock<Matrix<1, 4, f64>>,
}

impl Manualinpute8ec8Component {
    pub fn new(_context: &Context) -> Self {
        let manual_input_param =
            <FfiInputBlock<ManualControlInput> as InputBlock>::Parameters::new();
        let manual_input_block = FfiInputBlock::default();

        // TimeStamp
        let time_stamp_e8ec9_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let time_stamp_e8ec9 = ComponentOutputBlock::default();

        let vector_transpose_param =
            <TransposeBlock<Matrix<1, 12, f64>> as ProcessBlock>::Parameters::new();
        let vector_transpose = TransposeBlock::default();

        // Sticks
        let sticks_e8ecb_param =
            <ComponentOutputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters::new();
        let sticks_e8ecb = ComponentOutputBlock::default();

        Manualinpute8ec8Component {
            last_time_s: -1.0,
            manual_input_param,
            manual_input_block,
            time_stamp_e8ec9_param,
            time_stamp_e8ec9,
            vector_transpose_param,
            vector_transpose,
            sticks_e8ecb_param,
            sticks_e8ecb,
        }
    }

    pub fn run(&mut self, context: &mut Context) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // GpioInput1
        let manual_input = self
            .manual_input_block
            .input(&self.manual_input_param, &runtime_ctx);

        // TimeStamp
        let _ = self.time_stamp_e8ec9.process(
            &self.time_stamp_e8ec9_param,
            &runtime_ctx,
            manual_input.0,
        );

        // Vector Transpose
        let vector_transpose = self.vector_transpose.process(
            &self.vector_transpose_param,
            &runtime_ctx,
            manual_input.3,
        );

        // Sticks
        let _ = self
            .sticks_e8ecb
            .process(&self.sticks_e8ecb_param, &runtime_ctx, vector_transpose);

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Atan2e8fb7Component {
    last_time_s: f64,
    y_e8fb8_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    y_e8fb8: ComponentInputBlock<f64>,
    x_e8fb9_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    x_e8fb9: ComponentInputBlock<f64>,
    rust_code1_e8fcf_param: <RustCodeBlock<f64> as ProcessBlock>::Parameters,
    rust_code1_e8fcf: RustCodeBlock<f64>,
    component_output1_e8fd0_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    component_output1_e8fd0: ComponentOutputBlock<f64>,
}

impl Atan2e8fb7Component {
    pub fn new(_context: &Context) -> Self {
        // y
        let y_e8fb8_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let y_e8fb8 = ComponentInputBlock::default();

        // x
        let x_e8fb9_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let x_e8fb9 = ComponentInputBlock::default();

        // RustCode1
        let rust_code1_e8fcf_param = <RustCodeBlock<f64> as ProcessBlock>::Parameters::new();
        let rust_code1_e8fcf = RustCodeBlock::default();

        // ComponentOutput1
        let component_output1_e8fd0_param =
            <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let component_output1_e8fd0 = ComponentOutputBlock::default();

        Atan2e8fb7Component {
            last_time_s: -1.0,
            y_e8fb8_param,
            y_e8fb8,
            x_e8fb9_param,
            x_e8fb9,
            rust_code1_e8fcf_param,
            rust_code1_e8fcf,
            component_output1_e8fd0_param,
            component_output1_e8fd0,
        }
    }

    pub fn run(&mut self, context: &mut Context, y_e8fb8: f64, x_e8fb9: f64) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // y
        let y_e8fb8_0 = self
            .y_e8fb8
            .process(&self.y_e8fb8_param, &runtime_ctx, y_e8fb8);

        // x
        let x_e8fb9_0 = self
            .x_e8fb9
            .process(&self.x_e8fb9_param, &runtime_ctx, x_e8fb9);

        let rust_code1_e8fcf_val = user_functions::rust_fn_e29b(y_e8fb8_0, x_e8fb9_0);
        // RustCode1
        let rust_code1_e8fcf_0 = self.rust_code1_e8fcf.process(
            &self.rust_code1_e8fcf_param,
            &runtime_ctx,
            rust_code1_e8fcf_val,
        );

        // ComponentOutput1
        let _ = self.component_output1_e8fd0.process(
            &self.component_output1_e8fd0_param,
            &runtime_ctx,
            rust_code1_e8fcf_0,
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Atan4e8fe6Component {
    last_time_s: f64,
    y_e8fe7_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    y_e8fe7: ComponentInputBlock<f64>,
    x_e8fe8_param: <ComponentInputBlock<f64> as ProcessBlock>::Parameters,
    x_e8fe8: ComponentInputBlock<f64>,
    rust_code1_e8fe9_param: <RustCodeBlock<f64> as ProcessBlock>::Parameters,
    rust_code1_e8fe9: RustCodeBlock<f64>,
    component_output1_e8fea_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    component_output1_e8fea: ComponentOutputBlock<f64>,
}

impl Atan4e8fe6Component {
    pub fn new(_context: &Context) -> Self {
        // y
        let y_e8fe7_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let y_e8fe7 = ComponentInputBlock::default();

        // x
        let x_e8fe8_param = <ComponentInputBlock<f64> as ProcessBlock>::Parameters::new();
        let x_e8fe8 = ComponentInputBlock::default();

        // RustCode1
        let rust_code1_e8fe9_param = <RustCodeBlock<f64> as ProcessBlock>::Parameters::new();
        let rust_code1_e8fe9 = RustCodeBlock::default();

        // ComponentOutput1
        let component_output1_e8fea_param =
            <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let component_output1_e8fea = ComponentOutputBlock::default();

        Atan4e8fe6Component {
            last_time_s: -1.0,
            y_e8fe7_param,
            y_e8fe7,
            x_e8fe8_param,
            x_e8fe8,
            rust_code1_e8fe9_param,
            rust_code1_e8fe9,
            component_output1_e8fea_param,
            component_output1_e8fea,
        }
    }

    pub fn run(&mut self, context: &mut Context, y_e8fe7: f64, x_e8fe8: f64) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // y
        let y_e8fe7_0 = self
            .y_e8fe7
            .process(&self.y_e8fe7_param, &runtime_ctx, y_e8fe7);

        // x
        let x_e8fe8_0 = self
            .x_e8fe8
            .process(&self.x_e8fe8_param, &runtime_ctx, x_e8fe8);

        let rust_code1_e8fe9_val = user_functions::rust_fn_e29b(y_e8fe7_0, x_e8fe8_0);
        // RustCode1
        let rust_code1_e8fe9_0 = self.rust_code1_e8fe9.process(
            &self.rust_code1_e8fe9_param,
            &runtime_ctx,
            rust_code1_e8fe9_val,
        );

        // ComponentOutput1
        let _ = self.component_output1_e8fea.process(
            &self.component_output1_e8fea_param,
            &runtime_ctx,
            rust_code1_e8fe9_0,
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Quattorpye8f12Component {
    last_time_s: f64,
    quat_e8f13_param: <ComponentInputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    quat_e8f13: ComponentInputBlock<Matrix<1, 4, f64>>,
    vector_index3_e8fa5_param:
        <VectorIndexBlock<4, f64, Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    vector_index3_e8fa5: VectorIndexBlock<4, f64, Matrix<1, 4, f64>>,
    qy_qz_e8fab_param: <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters,
    qy_qz_e8fab: ProductBlock<(f64, f64), ComponentWise>,
    qw_qx_e8fae_param: <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters,
    qw_qx_e8fae: ProductBlock<(f64, f64), ComponentWise>,
    sum3_e8ff5_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum3_e8ff5: SumBlock<(f64, f64)>,
    gain1_e8ff3_param: <GainBlock<f64, f64> as ProcessBlock>::Parameters,
    gain1_e8ff3: GainBlock<f64, f64>,
    constant4_e8ff9_param: <ConstantBlock<f64> as GeneratorBlock>::Parameters,
    constant4_e8ff9: ConstantBlock<f64>,
    exponent1_e8fff_param: <ExponentBlock<f64> as ProcessBlock>::Parameters,
    exponent1_e8fff: ExponentBlock<f64>,
    exponent2_e9000_param: <ExponentBlock<f64> as ProcessBlock>::Parameters,
    exponent2_e9000: ExponentBlock<f64>,
    sum5_e9001_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum5_e9001: SumBlock<(f64, f64)>,
    gain2_e8ffd_param: <GainBlock<f64, f64> as ProcessBlock>::Parameters,
    gain2_e8ffd: GainBlock<f64, f64>,
    sum4_e8ffa_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum4_e8ffa: SumBlock<(f64, f64)>,
    atan2e8fb7_component: Atan2e8fb7Component,
    roll_e8f14_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    roll_e8f14: ComponentOutputBlock<f64>,
    constant3_e8ff0_param: <ConstantBlock<f64> as GeneratorBlock>::Parameters,
    constant3_e8ff0: ConstantBlock<f64>,
    qw_qy_e8fa8_param: <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters,
    qw_qy_e8fa8: ProductBlock<(f64, f64), ComponentWise>,
    qx_qz_86e04_param: <ProductBlock<(f64, f64, f64), ComponentWise> as ProcessBlock>::Parameters,
    qx_qz_86e04: ProductBlock<(f64, f64, f64), ComponentWise>,
    sum9_e9023_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum9_e9023: SumBlock<(f64, f64)>,
    gain6_e9027_param: <GainBlock<f64, f64> as ProcessBlock>::Parameters,
    gain6_e9027: GainBlock<f64, f64>,
    constant11_e902b_param: <ConstantBlock<f64> as GeneratorBlock>::Parameters,
    constant11_e902b: ConstantBlock<f64>,
    sum10_e9029_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum10_e9029: SumBlock<(f64, f64)>,
    exponent5_e901c_param: <ExponentBlock<f64> as ProcessBlock>::Parameters,
    exponent5_e901c: ExponentBlock<f64>,
    constant16_e902f_param: <ConstantBlock<f64> as GeneratorBlock>::Parameters,
    constant16_e902f: ConstantBlock<f64>,
    sum11_e902a_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum11_e902a: SumBlock<(f64, f64)>,
    exponent6_e901d_param: <ExponentBlock<f64> as ProcessBlock>::Parameters,
    exponent6_e901d: ExponentBlock<f64>,
    atan4e8fe6_component: Atan4e8fe6Component,
    gain5_e901e_param: <GainBlock<f64, f64> as ProcessBlock>::Parameters,
    gain5_e901e: GainBlock<f64, f64>,
    sum2_e8fe5_param: <SumBlock<(f64, f64)> as ProcessBlock>::Parameters,
    sum2_e8fe5: SumBlock<(f64, f64)>,
    pitch_e8f15_param: <ComponentOutputBlock<f64> as ProcessBlock>::Parameters,
    pitch_e8f15: ComponentOutputBlock<f64>,
}

impl Quattorpye8f12Component {
    pub fn new(context: &Context) -> Self {
        // Quat
        let quat_e8f13_param =
            <ComponentInputBlock<Matrix<4, 1, f64>> as ProcessBlock>::Parameters::new();
        let quat_e8f13 = ComponentInputBlock::default();

        let vector_index3_e8fa5_select_indexes = Vec::from([
            String::from("Scalar:0"),
            String::from("Scalar:1"),
            String::from("Scalar:2"),
            String::from("Scalar:3"),
        ]);

        // VectorIndex3
        let vector_index3_e8fa5_param =
            <VectorIndexBlock<4, f64, Matrix<4, 1, f64>> as ProcessBlock>::Parameters::new(
                &vector_index3_e8fa5_select_indexes,
            );
        let vector_index3_e8fa5 = VectorIndexBlock::default();

        let qy_qz_e8fab_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // qy_qz
        let qy_qz_e8fab_param =
            <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters::new(
                qy_qz_e8fab_gains.to_pass(),
            );
        let qy_qz_e8fab = ProductBlock::default();

        let qw_qx_e8fae_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // qw_qx
        let qw_qx_e8fae_param =
            <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters::new(
                qw_qx_e8fae_gains.to_pass(),
            );
        let qw_qx_e8fae = ProductBlock::default();

        let sum3_e8ff5_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // Sum3
        let sum3_e8ff5_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum3_e8ff5_gains.to_pass());
        let sum3_e8ff5 = SumBlock::default();

        let gain1_e8ff3_gain = 2.000000;

        // Gain1
        let gain1_e8ff3_param =
            <GainBlock<f64, f64> as ProcessBlock>::Parameters::new(gain1_e8ff3_gain);
        let gain1_e8ff3 = GainBlock::default();

        let constant4_e8ff9_value = 1.000000;

        let constant4_e8ff9_ic = BlockData::from_element(1, 1, constant4_e8ff9_value);

        // Constant4
        let constant4_e8ff9_param =
            <ConstantBlock<f64> as GeneratorBlock>::Parameters::new(constant4_e8ff9_ic.to_pass());
        let constant4_e8ff9 = ConstantBlock::default();

        let exponent1_e8fff_coefficient = 2.000000;
        let exponent1_e8fff_preserve_sign = 0.000000;

        // Exponent1
        let exponent1_e8fff_param = <ExponentBlock<f64> as ProcessBlock>::Parameters::new(
            exponent1_e8fff_coefficient,
            exponent1_e8fff_preserve_sign,
        );
        let exponent1_e8fff = ExponentBlock::default();

        let exponent2_e9000_coefficient = 2.000000;
        let exponent2_e9000_preserve_sign = 0.000000;

        // Exponent2
        let exponent2_e9000_param = <ExponentBlock<f64> as ProcessBlock>::Parameters::new(
            exponent2_e9000_coefficient,
            exponent2_e9000_preserve_sign,
        );
        let exponent2_e9000 = ExponentBlock::default();

        let sum5_e9001_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Sum5
        let sum5_e9001_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum5_e9001_gains.to_pass());
        let sum5_e9001 = SumBlock::default();

        let gain2_e8ffd_gain = 2.000000;

        // Gain2
        let gain2_e8ffd_param =
            <GainBlock<f64, f64> as ProcessBlock>::Parameters::new(gain2_e8ffd_gain);
        let gain2_e8ffd = GainBlock::default();

        let sum4_e8ffa_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Sum4
        let sum4_e8ffa_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum4_e8ffa_gains.to_pass());
        let sum4_e8ffa = SumBlock::default();

        let atan2e8fb7_component = Atan2e8fb7Component::new(context);

        // Roll
        let roll_e8f14_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let roll_e8f14 = ComponentOutputBlock::default();

        let constant3_e8ff0_value = context.gds.half_pi_e8fc9_e8ff1.clone();

        let constant3_e8ff0_ic = BlockData::from_element(1, 1, constant3_e8ff0_value);

        // Constant3
        let constant3_e8ff0_param =
            <ConstantBlock<f64> as GeneratorBlock>::Parameters::new(constant3_e8ff0_ic.to_pass());
        let constant3_e8ff0 = ConstantBlock::default();

        let qw_qy_e8fa8_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // qw_qy
        let qw_qy_e8fa8_param =
            <ProductBlock<(f64, f64), ComponentWise> as ProcessBlock>::Parameters::new(
                qw_qy_e8fa8_gains.to_pass(),
            );
        let qw_qy_e8fa8 = ProductBlock::default();

        let qx_qz_86e04_gains = BlockData::new(1, 3, &[1.0, 1.0, 1.0]);

        // qx_qz
        let qx_qz_86e04_param =
            <ProductBlock<(f64, f64, f64), ComponentWise> as ProcessBlock>::Parameters::new(
                qx_qz_86e04_gains.to_pass(),
            );
        let qx_qz_86e04 = ProductBlock::default();

        let sum9_e9023_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Sum9
        let sum9_e9023_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum9_e9023_gains.to_pass());
        let sum9_e9023 = SumBlock::default();

        let gain6_e9027_gain = 2.000000;

        // Gain6
        let gain6_e9027_param =
            <GainBlock<f64, f64> as ProcessBlock>::Parameters::new(gain6_e9027_gain);
        let gain6_e9027 = GainBlock::default();

        let constant11_e902b_value = 1.000000;

        let constant11_e902b_ic = BlockData::from_element(1, 1, constant11_e902b_value);

        // Constant11
        let constant11_e902b_param =
            <ConstantBlock<f64> as GeneratorBlock>::Parameters::new(constant11_e902b_ic.to_pass());
        let constant11_e902b = ConstantBlock::default();

        let sum10_e9029_gains = BlockData::new(1, 2, &[1.0, 1.0]);

        // Sum10
        let sum10_e9029_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum10_e9029_gains.to_pass());
        let sum10_e9029 = SumBlock::default();

        let exponent5_e901c_coefficient = 0.500000;
        let exponent5_e901c_preserve_sign = 0.000000;

        // Exponent5
        let exponent5_e901c_param = <ExponentBlock<f64> as ProcessBlock>::Parameters::new(
            exponent5_e901c_coefficient,
            exponent5_e901c_preserve_sign,
        );
        let exponent5_e901c = ExponentBlock::default();

        let constant16_e902f_value = 1.000000;

        let constant16_e902f_ic = BlockData::from_element(1, 1, constant16_e902f_value);

        // Constant16
        let constant16_e902f_param =
            <ConstantBlock<f64> as GeneratorBlock>::Parameters::new(constant16_e902f_ic.to_pass());
        let constant16_e902f = ConstantBlock::default();

        let sum11_e902a_gains = BlockData::new(1, 2, &[-1.0, 1.0]);

        // Sum11
        let sum11_e902a_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum11_e902a_gains.to_pass());
        let sum11_e902a = SumBlock::default();

        let exponent6_e901d_coefficient = 0.500000;
        let exponent6_e901d_preserve_sign = 0.000000;

        // Exponent6
        let exponent6_e901d_param = <ExponentBlock<f64> as ProcessBlock>::Parameters::new(
            exponent6_e901d_coefficient,
            exponent6_e901d_preserve_sign,
        );
        let exponent6_e901d = ExponentBlock::default();

        let atan4e8fe6_component = Atan4e8fe6Component::new(context);

        let gain5_e901e_gain = 2.000000;

        // Gain5
        let gain5_e901e_param =
            <GainBlock<f64, f64> as ProcessBlock>::Parameters::new(gain5_e901e_gain);
        let gain5_e901e = GainBlock::default();

        let sum2_e8fe5_gains = BlockData::new(1, 2, &[1.0, -1.0]);

        // Sum2
        let sum2_e8fe5_param =
            <SumBlock<(f64, f64)> as ProcessBlock>::Parameters::new(sum2_e8fe5_gains.to_pass());
        let sum2_e8fe5 = SumBlock::default();

        // Pitch
        let pitch_e8f15_param = <ComponentOutputBlock<f64> as ProcessBlock>::Parameters::new();
        let pitch_e8f15 = ComponentOutputBlock::default();

        Quattorpye8f12Component {
            last_time_s: -1.0,
            quat_e8f13_param,
            quat_e8f13,
            vector_index3_e8fa5_param,
            vector_index3_e8fa5,
            qy_qz_e8fab_param,
            qy_qz_e8fab,
            qw_qx_e8fae_param,
            qw_qx_e8fae,
            sum3_e8ff5_param,
            sum3_e8ff5,
            gain1_e8ff3_param,
            gain1_e8ff3,
            constant4_e8ff9_param,
            constant4_e8ff9,
            exponent1_e8fff_param,
            exponent1_e8fff,
            exponent2_e9000_param,
            exponent2_e9000,
            sum5_e9001_param,
            sum5_e9001,
            gain2_e8ffd_param,
            gain2_e8ffd,
            sum4_e8ffa_param,
            sum4_e8ffa,
            atan2e8fb7_component,
            roll_e8f14_param,
            roll_e8f14,
            constant3_e8ff0_param,
            constant3_e8ff0,
            qw_qy_e8fa8_param,
            qw_qy_e8fa8,
            qx_qz_86e04_param,
            qx_qz_86e04,
            sum9_e9023_param,
            sum9_e9023,
            gain6_e9027_param,
            gain6_e9027,
            constant11_e902b_param,
            constant11_e902b,
            sum10_e9029_param,
            sum10_e9029,
            exponent5_e901c_param,
            exponent5_e901c,
            constant16_e902f_param,
            constant16_e902f,
            sum11_e902a_param,
            sum11_e902a,
            exponent6_e901d_param,
            exponent6_e901d,
            atan4e8fe6_component,
            gain5_e901e_param,
            gain5_e901e,
            sum2_e8fe5_param,
            sum2_e8fe5,
            pitch_e8f15_param,
            pitch_e8f15,
        }
    }

    pub fn run(&mut self, context: &mut Context, quat_e8f13: &Matrix<1, 4, f64>) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // Quat
        let quat_e8f13_0 =
            self.quat_e8f13
                .process(&self.quat_e8f13_param, &runtime_ctx, quat_e8f13);

        // VectorIndex3
        let (
            vector_index3_e8fa5_0,
            vector_index3_e8fa5_1,
            vector_index3_e8fa5_2,
            vector_index3_e8fa5_3,
        ) = self.vector_index3_e8fa5.process(
            &self.vector_index3_e8fa5_param,
            &runtime_ctx,
            &quat_e8f13_0,
        );

        // qy_qz
        let qy_qz_e8fab_0 = self.qy_qz_e8fab.process(
            &self.qy_qz_e8fab_param,
            &runtime_ctx,
            (vector_index3_e8fa5_1, vector_index3_e8fa5_3),
        );

        // qw_qx
        let qw_qx_e8fae_0 = self.qw_qx_e8fae.process(
            &self.qw_qx_e8fae_param,
            &runtime_ctx,
            (vector_index3_e8fa5_0, vector_index3_e8fa5_1),
        );

        // Sum3
        let sum3_e8ff5_0 = self.sum3_e8ff5.process(
            &self.sum3_e8ff5_param,
            &runtime_ctx,
            (qw_qx_e8fae_0, qy_qz_e8fab_0),
        );

        // Gain1
        let gain1_e8ff3_0 =
            self.gain1_e8ff3
                .process(&self.gain1_e8ff3_param, &runtime_ctx, sum3_e8ff5_0);

        // Constant4
        let constant4_e8ff9_0 = self
            .constant4_e8ff9
            .generate(&self.constant4_e8ff9_param, &runtime_ctx);

        // Exponent1
        let exponent1_e8fff_0 = self.exponent1_e8fff.process(
            &self.exponent1_e8fff_param,
            &runtime_ctx,
            vector_index3_e8fa5_1,
        );

        // Exponent2
        let exponent2_e9000_0 = self.exponent2_e9000.process(
            &self.exponent2_e9000_param,
            &runtime_ctx,
            vector_index3_e8fa5_2,
        );

        // Sum5
        let sum5_e9001_0 = self.sum5_e9001.process(
            &self.sum5_e9001_param,
            &runtime_ctx,
            (exponent1_e8fff_0, exponent2_e9000_0),
        );

        // Gain2
        let gain2_e8ffd_0 =
            self.gain2_e8ffd
                .process(&self.gain2_e8ffd_param, &runtime_ctx, sum5_e9001_0);

        // Sum4
        let sum4_e8ffa_0 = self.sum4_e8ffa.process(
            &self.sum4_e8ffa_param,
            &runtime_ctx,
            (constant4_e8ff9_0, gain2_e8ffd_0),
        );

        // Component: atan2
        self.atan2e8fb7_component
            .run(context, gain1_e8ff3_0, sum4_e8ffa_0);

        // Roll
        let _ = self.roll_e8f14.process(
            &self.roll_e8f14_param,
            &runtime_ctx,
            self.atan2e8fb7_component
                .component_output1_e8fd0
                .data
                .to_pass(),
        );

        // Constant3
        let constant3_e8ff0_0 = self
            .constant3_e8ff0
            .generate(&self.constant3_e8ff0_param, &runtime_ctx);

        // qw_qy
        let qw_qy_e8fa8_0 = self.qw_qy_e8fa8.process(
            &self.qw_qy_e8fa8_param,
            &runtime_ctx,
            (vector_index3_e8fa5_0, vector_index3_e8fa5_2),
        );

        // qx_qz
        let qx_qy1_86e04_0 = self.qx_qz_86e04.process(
            &self.qx_qz_86e04_param,
            &runtime_ctx,
            (vector_index3_e8fa5_1, 0.0, vector_index3_e8fa5_3),
        );

        // Sum9
        let sum9_e9023_0 = self.sum9_e9023.process(
            &self.sum9_e9023_param,
            &runtime_ctx,
            (qw_qy_e8fa8_0, qx_qy1_86e04_0),
        );

        // Gain6
        let gain6_e9027_0 =
            self.gain6_e9027
                .process(&self.gain6_e9027_param, &runtime_ctx, sum9_e9023_0);

        // Constant11
        let constant11_e902b_0 = self
            .constant11_e902b
            .generate(&self.constant11_e902b_param, &runtime_ctx);

        // Sum10
        let sum10_e9029_0 = self.sum10_e9029.process(
            &self.sum10_e9029_param,
            &runtime_ctx,
            (constant11_e902b_0, gain6_e9027_0),
        );

        // Exponent5
        let exponent5_e901c_0 =
            self.exponent5_e901c
                .process(&self.exponent5_e901c_param, &runtime_ctx, sum10_e9029_0);

        // Constant16
        let constant16_e902f_0 = self
            .constant16_e902f
            .generate(&self.constant16_e902f_param, &runtime_ctx);

        // Sum11
        let sum11_e902a_0 = self.sum11_e902a.process(
            &self.sum11_e902a_param,
            &runtime_ctx,
            (gain6_e9027_0, constant16_e902f_0),
        );

        // Exponent6
        let exponent6_e901d_0 =
            self.exponent6_e901d
                .process(&self.exponent6_e901d_param, &runtime_ctx, sum11_e902a_0);

        // Component: atan4
        self.atan4e8fe6_component
            .run(context, exponent5_e901c_0, exponent6_e901d_0);

        // Gain5
        let gain5_e901e_0 = self.gain5_e901e.process(
            &self.gain5_e901e_param,
            &runtime_ctx,
            self.atan4e8fe6_component
                .component_output1_e8fea
                .data
                .to_pass(),
        );

        // Sum2
        let sum2_e8fe5_0 = self.sum2_e8fe5.process(
            &self.sum2_e8fe5_param,
            &runtime_ctx,
            (gain5_e901e_0, constant3_e8ff0_0),
        );

        // Pitch
        let _ = self
            .pitch_e8f15
            .process(&self.pitch_e8f15_param, &runtime_ctx, sum2_e8fe5_0);

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Vehicleodometerye8edaComponent {
    last_time_s: f64,
    vehicle_odometry_param: <FfiInputBlock<VehicleOdometry> as InputBlock>::Parameters,
    vehicle_odometry_block: FfiInputBlock<VehicleOdometry>,
    vector_transpose_1_param: <TransposeBlock<Matrix<4, 1, f64>> as ProcessBlock>::Parameters,
    vector_transpose_1: TransposeBlock<Matrix<4, 1, f64>>,
    q_e8edd_param: <ComponentOutputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    q_e8edd: ComponentOutputBlock<Matrix<1, 4, f64>>,
    vector_transpose_2_param: <TransposeBlock<Matrix<3, 1, f64>> as ProcessBlock>::Parameters,
    vector_transpose_2: TransposeBlock<Matrix<3, 1, f64>>,
    angular_velocity_e8ee1_param:
        <ComponentOutputBlock<Matrix<1, 3, f64>> as ProcessBlock>::Parameters,
    angular_velocity_e8ee1: ComponentOutputBlock<Matrix<1, 3, f64>>,
}

impl Vehicleodometerye8edaComponent {
    pub fn new(_context: &Context) -> Self {
        let vehicle_odometry_param =
            <FfiInputBlock<VehicleOdometry> as InputBlock>::Parameters::new();
        let vehicle_odometry_block = FfiInputBlock::default();

        // VectorTranspose1
        let vector_transpose_1_param =
            <TransposeBlock<Matrix<4, 1, f64>> as ProcessBlock>::Parameters::new();
        let vector_transpose_1 = TransposeBlock::default();

        // q
        let q_e8edd_param =
            <ComponentOutputBlock<Matrix<1, 4, f64>> as ProcessBlock>::Parameters::new();
        let q_e8edd = ComponentOutputBlock::default();

        // Vector Transpose2
        let vector_transpose_2_param =
            <TransposeBlock<Matrix<3, 1, f64>> as ProcessBlock>::Parameters::new();
        let vector_transpose_2 = TransposeBlock::default();

        // Angular Velocity
        let angular_velocity_e8ee1_param =
            <ComponentOutputBlock<Matrix<1, 3, f64>> as ProcessBlock>::Parameters::new();
        let angular_velocity_e8ee1 = ComponentOutputBlock::default();

        Vehicleodometerye8edaComponent {
            last_time_s: -1.0,
            vehicle_odometry_param,
            vehicle_odometry_block,
            vector_transpose_1_param,
            vector_transpose_1,
            q_e8edd_param,
            q_e8edd,
            vector_transpose_2_param,
            vector_transpose_2,
            angular_velocity_e8ee1_param,
            angular_velocity_e8ee1,
        }
    }

    pub fn run(&mut self, context: &mut Context) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        let vehicle_odometry = self
            .vehicle_odometry_block
            .input(&self.vehicle_odometry_param, &runtime_ctx);

        let transpose_1 = self.vector_transpose_1.process(
            &self.vector_transpose_1_param,
            &runtime_ctx,
            vehicle_odometry.2,
        );

        // q
        let _ = self
            .q_e8edd
            .process(&self.q_e8edd_param, &runtime_ctx, transpose_1);

        let transpose_2 = self.vector_transpose_2.process(
            &self.vector_transpose_2_param,
            &runtime_ctx,
            vehicle_odometry.4,
        );

        // Angular Velocity
        let _ = self.angular_velocity_e8ee1.process(
            &self.angular_velocity_e8ee1_param,
            &runtime_ctx,
            transpose_2,
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct Main48246State {
    last_time_s: f64,
    vehicleodometerye8eda_component: Vehicleodometerye8edaComponent,
    manualinpute8ec8_component: Manualinpute8ec8Component,
    control_split_out_e8f18_param:
        <VectorIndexBlock<4, f64, Matrix<1, 4, f64>> as ProcessBlock>::Parameters,
    control_split_out_e8f18: VectorIndexBlock<4, f64, Matrix<1, 4, f64>>,
    deadband1_e8f1c_param: <DeadbandBlock<f64> as ProcessBlock>::Parameters,
    deadband1_e8f1c: DeadbandBlock<f64>,
    yaw_rate_e8f19_param: <VectorIndexBlock<1, f64, Matrix<1, 3, f64>> as ProcessBlock>::Parameters,
    yaw_rate_e8f19: VectorIndexBlock<1, f64, Matrix<1, 3, f64>>,
    deadband2_e8f1e_param: <DeadbandBlock<f64> as ProcessBlock>::Parameters,
    deadband2_e8f1e: DeadbandBlock<f64>,
    deadband3_e8f1f_param: <DeadbandBlock<f64> as ProcessBlock>::Parameters,
    deadband3_e8f1f: DeadbandBlock<f64>,
    deadband4_e8f20_param: <DeadbandBlock<f64> as ProcessBlock>::Parameters,
    deadband4_e8f20: DeadbandBlock<f64>,
    quattorpye8f12_component: Quattorpye8f12Component,
    attitudecontrollere8f24_component: Attitudecontrollere8f24Component,
    actuatormixere8f47_component: Actuatormixere8f47Component,
    vector_merge2_e8f68_param:
        <VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)> as ProcessBlock>::Parameters,
    vector_merge2_e8f68: VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)>,
    actuatoroutpute8ed1_component: Actuatoroutpute8ed1Component,
}

impl Main48246State {
    pub fn new(context: &Context) -> Self {
        let vehicleodometerye8eda_component = Vehicleodometerye8edaComponent::new(context);
        let manualinpute8ec8_component = Manualinpute8ec8Component::new(context);

        let control_split_out_e8f18_select_indexes = Vec::from([
            String::from("Scalar:0"),
            String::from("Scalar:1"),
            String::from("Scalar:2"),
            String::from("Scalar:3"),
        ]);

        // Control Split Out
        let control_split_out_e8f18_param =
            <VectorIndexBlock<4, f64, Matrix<1, 4, f64>> as ProcessBlock>::Parameters::new(
                &control_split_out_e8f18_select_indexes,
            );
        let control_split_out_e8f18 = VectorIndexBlock::default();

        let deadband1_e8f1c_lower_limit = -0.100000;
        let deadband1_e8f1c_upper_limit = 0.100000;

        // Deadband1
        let deadband1_e8f1c_param = <DeadbandBlock<f64> as ProcessBlock>::Parameters::new(
            deadband1_e8f1c_lower_limit,
            deadband1_e8f1c_upper_limit,
        );
        let deadband1_e8f1c = DeadbandBlock::default();

        let yaw_rate_e8f19_select_indexes = Vec::from([String::from("Scalar:2")]);

        // YawRate
        let yaw_rate_e8f19_param =
            <VectorIndexBlock<1, f64, Matrix<1, 3, f64>> as ProcessBlock>::Parameters::new(
                &yaw_rate_e8f19_select_indexes,
            );
        let yaw_rate_e8f19 = VectorIndexBlock::default();

        let deadband2_e8f1e_lower_limit = -0.100000;
        let deadband2_e8f1e_upper_limit = 0.100000;

        // Deadband2
        let deadband2_e8f1e_param = <DeadbandBlock<f64> as ProcessBlock>::Parameters::new(
            deadband2_e8f1e_lower_limit,
            deadband2_e8f1e_upper_limit,
        );
        let deadband2_e8f1e = DeadbandBlock::default();

        let deadband3_e8f1f_lower_limit = -0.100000;
        let deadband3_e8f1f_upper_limit = 0.100000;

        // Deadband3
        let deadband3_e8f1f_param = <DeadbandBlock<f64> as ProcessBlock>::Parameters::new(
            deadband3_e8f1f_lower_limit,
            deadband3_e8f1f_upper_limit,
        );
        let deadband3_e8f1f = DeadbandBlock::default();

        let deadband4_e8f20_lower_limit = -0.100000;
        let deadband4_e8f20_upper_limit = 0.100000;

        // Deadband4
        let deadband4_e8f20_param = <DeadbandBlock<f64> as ProcessBlock>::Parameters::new(
            deadband4_e8f20_lower_limit,
            deadband4_e8f20_upper_limit,
        );
        let deadband4_e8f20 = DeadbandBlock::default();

        let quattorpye8f12_component = Quattorpye8f12Component::new(context);
        let attitudecontrollere8f24_component = Attitudecontrollere8f24Component::new(context);
        let actuatormixere8f47_component = Actuatormixere8f47Component::new(context);

        // VectorMerge2
        let vector_merge2_e8f68_param = <VectorMergeBlock<Matrix<1, 4, f64>, (f64, f64, f64, f64)> as ProcessBlock>::Parameters::new();
        let vector_merge2_e8f68 = VectorMergeBlock::default();

        let actuatoroutpute8ed1_component = Actuatoroutpute8ed1Component::new(context);

        Main48246State {
            last_time_s: -1.0,
            vehicleodometerye8eda_component,
            manualinpute8ec8_component,
            control_split_out_e8f18_param,
            control_split_out_e8f18,
            deadband1_e8f1c_param,
            deadband1_e8f1c,
            yaw_rate_e8f19_param,
            yaw_rate_e8f19,
            deadband2_e8f1e_param,
            deadband2_e8f1e,
            deadband3_e8f1f_param,
            deadband3_e8f1f,
            deadband4_e8f20_param,
            deadband4_e8f20,
            quattorpye8f12_component,
            attitudecontrollere8f24_component,
            actuatormixere8f47_component,
            vector_merge2_e8f68_param,
            vector_merge2_e8f68,
            actuatoroutpute8ed1_component,
        }
    }

    pub fn run(&mut self, context: &mut Context) {
        let app_time_s = context.app_time_s();
        let runtime_ctx = context.get_runtime_context();

        if self.last_time_s == -1.0 {
            self.last_time_s = app_time_s;
        }

        // Component: VehicleOdometery
        self.vehicleodometerye8eda_component.run(context);

        // Component: ManualInput
        self.manualinpute8ec8_component.run(context);

        // Control Split Out
        let (
            control_split_out_e8f18_0,
            control_split_out_e8f18_1,
            control_split_out_e8f18_2,
            control_split_out_e8f18_3,
        ) = self.control_split_out_e8f18.process(
            &self.control_split_out_e8f18_param,
            &runtime_ctx,
            &self.manualinpute8ec8_component.sticks_e8ecb.data.to_pass(),
        );

        // Deadband1
        let deadband1_e8f1c_0 = self.deadband1_e8f1c.process(
            &self.deadband1_e8f1c_param,
            &runtime_ctx,
            control_split_out_e8f18_0,
        );

        // YawRate
        let yaw_rate_e8f19_0 = self.yaw_rate_e8f19.process(
            &self.yaw_rate_e8f19_param,
            &runtime_ctx,
            &self
                .vehicleodometerye8eda_component
                .angular_velocity_e8ee1
                .data
                .to_pass(),
        );

        // Deadband2
        let deadband2_e8f1e_0 = self.deadband2_e8f1e.process(
            &self.deadband2_e8f1e_param,
            &runtime_ctx,
            control_split_out_e8f18_1,
        );

        // Deadband3
        let deadband3_e8f1f_0 = self.deadband3_e8f1f.process(
            &self.deadband3_e8f1f_param,
            &runtime_ctx,
            control_split_out_e8f18_2,
        );

        // Deadband4
        let deadband4_e8f20_0 = self.deadband4_e8f20.process(
            &self.deadband4_e8f20_param,
            &runtime_ctx,
            control_split_out_e8f18_3,
        );

        // Component: Quat to RPY
        self.quattorpye8f12_component.run(
            context,
            &self.vehicleodometerye8eda_component.q_e8edd.data.to_pass(),
        );

        // Component: Attitude Controller
        self.attitudecontrollere8f24_component.run(
            context,
            deadband2_e8f1e_0,
            deadband3_e8f1f_0,
            deadband4_e8f20_0,
            self.quattorpye8f12_component.pitch_e8f15.data.to_pass(),
            self.quattorpye8f12_component.roll_e8f14.data.to_pass(),
            yaw_rate_e8f19_0,
        );

        // Component: Actuator Mixer
        self.actuatormixere8f47_component.run(
            context,
            deadband1_e8f1c_0,
            self.attitudecontrollere8f24_component
                .tau_pitch_e8f2d
                .data
                .to_pass(),
            self.attitudecontrollere8f24_component
                .tau_roll_e8f2e
                .data
                .to_pass(),
            self.attitudecontrollere8f24_component
                .tau_yaw_e8f2c
                .data
                .to_pass(),
        );

        // VectorMerge2
        let vector_merge2_e8f68_0 = self.vector_merge2_e8f68.process(
            &self.vector_merge2_e8f68_param,
            &runtime_ctx,
            (
                self.actuatormixere8f47_component
                    .motor_1_e8f5a
                    .data
                    .to_pass(),
                self.actuatormixere8f47_component
                    .motor_2_e8f5b
                    .data
                    .to_pass(),
                self.actuatormixere8f47_component
                    .motor_3_e8f5c
                    .data
                    .to_pass(),
                self.actuatormixere8f47_component
                    .motor_4_e8f5d
                    .data
                    .to_pass(),
            ),
        );

        // Component: Actuator Output
        self.actuatoroutpute8ed1_component.run(
            context,
            &vector_merge2_e8f68_0,
            self.manualinpute8ec8_component
                .time_stamp_e8ec9
                .data
                .to_pass(),
        );

        self.last_time_s = app_time_s;
    }

    pub fn post_run(&mut self) {}
}

pub struct StateManager {
    pub current_state: State,
    pub main48246_state: Main48246State,
}

impl StateManager {
    pub fn run(&mut self, context: &mut Context) {
        match self.current_state {
            State::Main48246State => self.main48246_state.run(context),
        };
    }
}

pub struct GlobalDataStore {
    pub half_pi_e8fc9_e8ff1: f64,
}

impl GlobalDataStore {
    // Constructor
    pub fn new() -> GlobalDataStore {
        GlobalDataStore {
            half_pi_e8fc9_e8ff1: 1.5707963267948966,
        }
    }
}

//  ----- C interface methods ----- //

#[no_mangle]
pub extern "C" fn app_interface_new() -> *mut AppInterface {
    /*
    Allows users to create an AppInterface object, to control
    app execution from other languages.
    */
    // let pictorus_vars = get_pictorus_vars();
    // let diagram_params = get_diagram_params(&pictorus_vars);
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 131_072;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(addr_of_mut!(HEAP_MEM) as usize, HEAP_SIZE) }
    }

    let gds = GlobalDataStore::new();
    let io_manager = IoManager::new().expect("Unable to initialize IoManager!");
    let context = Context {
        gds,
        io_manager,
        runtime_context: pictorus_internal::RuntimeContext::new(100000),
    };

    let app_interface = AppInterface::new(context);

    Box::into_raw(Box::new(app_interface))
}

#[no_mangle]
pub extern "C" fn app_interface_free(app: *mut AppInterface) {
    /*
    Allows users to free an AppInterface object from memory from other languages.
    */

    if app.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(app);
    }
}

#[no_mangle]
pub extern "C" fn app_interface_update(app: *mut AppInterface, timestamp: u64) {
    /*
    Allows users to iterate one execution step for a given AppInterface
    */
    let app_interface = unsafe {
        assert!(!app.is_null());
        &mut *app
    };

    app_interface.context.update_app_time(timestamp);
    app_interface.update();
}
//  ------------------------------ //

pub struct IoManager {}

impl IoManager {
    #[allow(unused_mut)]
    pub fn new() -> Result<Self, PictorusError> {
        FfiProtocol::get_mut().subscribe_to_message(VehicleOdometry);
        FfiProtocol::get_mut().subscribe_to_message(ManualControlInput);
        FfiProtocol::get_mut().advertise_message(ActuatorMotors);
        FfiProtocol::get_mut().advertise_message(ActuatorArmed);
        Ok(Self {})
    }

    pub fn flush_inputs(&mut self) {}
}

pub struct AppInterface {
    state_manager: StateManager,
    context: Context,
    last_armed_time: u64,
}

impl AppInterface {
    pub fn new(context: Context) -> Self {
        let state_manager = StateManager {
            current_state: State::Main48246State,
            main48246_state: Main48246State::new(&context),
        };

        Self {
            state_manager,
            context,
            last_armed_time: 0,
        }
    }

    pub fn update(&mut self) {
        // Publish armed state at 2 Hz
        let app_time_us = self.context.app_time_us();
        if app_time_us - self.last_armed_time > 500_000 {
            FfiOutputBlock::<ActuatorArmed>::default().output(
                &<FfiOutputBlock<ActuatorArmed> as OutputBlock>::Parameters::new(),
                &self.context.get_runtime_context(),
                (1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            );
            self.last_armed_time = app_time_us;
        }

        self.state_manager.run(&mut self.context);

        self.context.io_manager.flush_inputs();
    }
}

pub struct Context {
    gds: GlobalDataStore,
    io_manager: IoManager,
    runtime_context: pictorus_internal::RuntimeContext,
}

impl Context {
    pub fn app_time_s(&self) -> f64 {
        self.runtime_context.app_time_s()
    }

    pub fn app_time_us(&self) -> u64 {
        self.runtime_context.app_time_us()
    }

    pub fn time(&self) -> Duration {
        self.runtime_context.time()
    }

    pub fn get_runtime_context(&self) -> pictorus_internal::RuntimeContext {
        self.runtime_context
    }

    pub fn update_app_time(&mut self, app_time_us: u64) {
        self.runtime_context.update_app_time(app_time_us);
    }
}

pub mod user_functions {
    // All user defined functions
    pub fn rust_fn_e29b(y: f64, x: f64) -> f64 {
        <f64 as ::num_traits::float::Float>::atan2(y, x)
    }
}
