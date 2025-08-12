use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Parameters for the VectorReshapeBlock
pub struct Parameters {}

impl Default for Parameters {
    fn default() -> Self {
        Self::new()
    }
}

impl Parameters {
    pub fn new() -> Self {
        Self {}
    }
}

/// Reshapes an input signal to a specified output shape.
///
/// If the input type is an (M, N) matrix, the output type MUST have dimensions such that M_in*N_in == M_out*N_out.
/// Accepts a scalar input, T, if the output is a Matrix<1, 1, T>.
pub struct VectorReshapeBlock<I, O> {
    pub data: OldBlockData,
    buffer: O,
    _phantom: core::marker::PhantomData<I>,
}

impl<I, O> Default for VectorReshapeBlock<I, O>
where
    O: Default + Pass,
    OldBlockData: FromPass<O>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<O>>::from_pass(O::default().as_by()),
            buffer: O::default(),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<const IROWS: usize, const ICOLS: usize, const OROWS: usize, const OCOLS: usize, T> ProcessBlock
    for VectorReshapeBlock<Matrix<IROWS, ICOLS, T>, Matrix<OROWS, OCOLS, T>>
where
    T: pictorus_traits::Scalar,
{
    type Inputs = Matrix<IROWS, ICOLS, T>;
    type Output = Matrix<OROWS, OCOLS, T>;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        const {
            assert!(
                IROWS * ICOLS == OROWS * OCOLS,
                "Input matrix dimensions do not match output matrix dimensions in VectorSortBlock"
            );
        }

        // Copy the input data into the buffer. Sizes are guaranteed to be the same
        self.buffer
            .data
            .as_flattened_mut()
            .copy_from_slice(input.data.as_flattened());
        self.data = <OldBlockData as FromPass<Self::Output>>::from_pass(&self.buffer);
        &self.buffer
    }
}

impl<T> ProcessBlock for VectorReshapeBlock<T, Matrix<1, 1, T>>
where
    T: pictorus_traits::Scalar,
{
    type Inputs = T;
    type Output = Matrix<1, 1, T>;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        self.buffer.data[0][0] = input;
        self.data = <OldBlockData as FromPass<Self::Output>>::from_pass(&self.buffer);
        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    /// This test should fail to compile due to the assertion test for the input and output matrix dimensions.
    /// ```compile_fail,E0080
    /// #[test]
    /// fn static_assert() {
    ///      let c = StubContext::default();
    ///      let mut block = VectorReshapeBlock::<Matrix<3, 3, f64>, Matrix<4, 4, f64>>::default();
    ///      let parameters = Parameters::new();
    ///
    ///     let input = Matrix {
    ///          data: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
    ///      };
    ///      let _output = block.process(&parameters, &c, &input);
    /// }
    /// ```
    macro_rules! impl_vector_reshape_tests {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_sort_scalar_ $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorReshapeBlock::<$type, Matrix<1, 1, $type>>::default();
                    let parameters = Parameters::new();

                    let input = [<3 $type>];

                    let output = block.process(&parameters, &c, input);

                    assert_eq!(output.data, [[[<3 $type>]]]);
                }

                #[test]
                fn [<test_vector_reshape_3x3_1x9 $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorReshapeBlock::<Matrix<3, 3, $type>, Matrix<1, 9, $type>>::default();
                    let parameters = Parameters::new();

                    let input = Matrix {
                        data: [[
                                [<3 $type>], [<6 $type>], [<9 $type>]],
                                [[<7 $type>], [<5 $type>], [<2 $type>]],
                                [[<1 $type>], [<10 $type>], [<8 $type>]
                            ]],
                    };

                    let output = block.process(&parameters, &c, &input).data;

                    assert_eq!(
                        output,
                        [
                            [[<3 $type>]],
                            [[<6 $type>]],
                            [[<9 $type>]],
                            [[<7 $type>]],
                            [[<5 $type>]],
                            [[<2 $type>]],
                            [[<1 $type>]],
                            [[<10 $type>]],
                            [[<8 $type>]]
                        ]
                    );
                }

                #[test]
                fn [<test_vector_reshape_3x2_2x3_ $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorReshapeBlock::<Matrix<3, 2, $type>, Matrix<2, 3, $type>>::default();
                    let parameters = Parameters::new();

                    // This test has redundant values (3 and 7)
                    let input = Matrix {
                        data: [[
                                [<3 $type>], [<7 $type>], [<9 $type>]],
                                [[<7 $type>], [<5 $type>], [<2 $type>],
                            ]],
                    };

                    let output = block.process(&parameters, &c, &input).data;

                    assert_eq!(
                        output,
                        [
                            [[<3 $type>], [<7 $type>]],
                            [[<9 $type>], [<7 $type>]],
                            [[<5 $type>], [<2 $type>]],
                        ]
                    );
                }

                #[test]
                fn [<test_vector_reshape_4x4_8x2 $type>]() {
                    // Matlab documentation example
                    let c = StubContext::default();
                    let mut block = VectorReshapeBlock::<Matrix<4, 4, $type>, Matrix<8, 2, $type>>::default();
                    let parameters = Parameters::new();

                    // This test has redundant values (3 and 7)
                    let input = Matrix {
                        data: [
                                [[<16 $type>], [<5 $type>], [<9 $type>], [<4 $type>]],
                                [[<2 $type>], [<11 $type>], [<7 $type>], [<14 $type>]],
                                [[<3 $type>], [<10 $type>], [<6 $type>], [<15 $type>]],
                                [[<13 $type>], [<8 $type>], [<12 $type>], [<1 $type>]],
                            ],
                    };

                    let output = block.process(&parameters, &c, &input).data;

                    assert_eq!(
                        output,
                        [
                            [[<16 $type>], [<5 $type>], [<9 $type>], [<4 $type>], [<2 $type>], [<11 $type>], [<7 $type>], [<14 $type>]],
                            [[<3 $type>], [<10 $type>], [<6 $type>], [<15 $type>], [<13 $type>], [<8 $type>], [<12 $type>], [<1 $type>]],
                        ]
                    );
                }
            }
        };
    }

    impl_vector_reshape_tests!(f64);
    impl_vector_reshape_tests!(f32);
    impl_vector_reshape_tests!(i32);
    impl_vector_reshape_tests!(u32);
    impl_vector_reshape_tests!(i16);
    impl_vector_reshape_tests!(u16);
    impl_vector_reshape_tests!(i8);
    impl_vector_reshape_tests!(u8);
}
