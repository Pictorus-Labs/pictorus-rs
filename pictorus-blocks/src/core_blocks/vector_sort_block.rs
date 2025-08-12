use core::cmp::Ordering;

use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

#[derive(strum::EnumString, PartialEq)]
pub enum VectorSortDirection {
    Ascending,
    Descending,
}

/// Parameters for the VectorSortBlock
pub struct Parameters {
    /// Direction of the sort, either Ascending or Descending
    pub direction: VectorSortDirection,
}

impl Parameters {
    pub fn new(direction: &str) -> Self {
        Self {
            direction: direction
                .parse()
                .expect("Failed to parse VectorSortDirection"),
        }
    }
}

/// Sorts all elements of the input signal into a vector output.
///
/// If the input type is an (M, N) matrix, the output type MUST be a (1, M*N) matrix or a panic will occur.
///
/// This block also accepts scalars, though the output will always be the input.
pub struct VectorSortBlock<I, O> {
    pub data: OldBlockData,
    buffer: O,
    _phantom: core::marker::PhantomData<I>,
}

impl<I, O> Default for VectorSortBlock<I, O>
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

macro_rules! impl_vector_sort {
    ($type:ty) => {
        impl<const IROWS: usize, const ICOLS: usize, const OCOLS: usize> ProcessBlock
            for VectorSortBlock<Matrix<IROWS, ICOLS, $type>, Matrix<1, OCOLS, $type>>
        {
            type Inputs = Matrix<IROWS, ICOLS, $type>;
            type Output = Matrix<1, OCOLS, $type>;
            type Parameters = Parameters;

            fn process(
                &mut self,
                parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                const { assert!(IROWS * ICOLS == OCOLS, "Input matrix dimensions do not match output matrix dimensions in VectorSortBlock"); }

                // Copy the input data into the buffer. Sizes are guaranteed to be the same from codegen
                self.buffer
                    .data
                    .as_flattened_mut()
                    .copy_from_slice(input.data.as_flattened());

                match parameters.direction {
                    VectorSortDirection::Ascending => {
                        self.buffer
                            .data
                            .as_flattened_mut()
                            .sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Less));
                    }
                    VectorSortDirection::Descending => {
                        self.buffer
                            .data
                            .as_flattened_mut()
                            .sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Less));
                    }
                }

                self.data = <OldBlockData as FromPass<Self::Output>>::from_pass(&self.buffer);
                &self.buffer
            }
        }

        impl ProcessBlock for VectorSortBlock<$type, $type> {
            type Inputs = $type;
            type Output = $type;
            type Parameters = Parameters;

            fn process(
                &mut self,
                _parameters: &Self::Parameters,
                _context: &dyn pictorus_traits::Context,
                input: PassBy<Self::Inputs>,
            ) -> PassBy<'_, Self::Output> {
                self.buffer = input;
                self.data = OldBlockData::from_scalar(self.buffer.into());
                self.buffer
            }
        }
    };
}

impl_vector_sort!(f64);
impl_vector_sort!(f32);
impl_vector_sort!(i32);
impl_vector_sort!(u32);
impl_vector_sort!(i16);
impl_vector_sort!(u16);
impl_vector_sort!(i8);
impl_vector_sort!(u8);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    /// This test should fail to compile due to the assertion test for the input and output matrix dimensions.
    /// ```compile_fail,E0080
    /// fn static_assert() {
    ///     let c = StubContext::default();
    ///     let mut block = VectorSortBlock::<Matrix<3, 3, f64>, Matrix<1, 8, f64>>::default();
    ///     let parameters = Parameters::new("Ascending");
    ///
    ///    let input = Matrix {
    ///         data: [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]],
    ///     };
    ///     let output = block.process(&parameters, &c, &input);
    /// }
    /// ```
    macro_rules! impl_vector_sort_tests {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_vector_sort_scalar_ $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorSortBlock::<$type, $type>::default();
                    let parameters = Parameters::new("Ascending");

                    let input = [<3 $type>];

                    let output = block.process(&parameters, &c, input);

                    assert_eq!(output, [<3 $type>]);
                }

                #[test]
                fn [<test_vector_sort_ascending_ $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorSortBlock::<Matrix<3, 3, $type>, Matrix<1, 9, $type>>::default();
                    let parameters = Parameters::new("Ascending");

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
                            [[<1 $type>]],
                            [[<2 $type>]],
                            [[<3 $type>]],
                            [[<5 $type>]],
                            [[<6 $type>]],
                            [[<7 $type>]],
                            [[<8 $type>]],
                            [[<9 $type>]],
                            [[<10 $type>]]
                        ]
                    );
                }

                #[test]
                fn [<test_vector_sort_descending_ $type>]() {
                    let c = StubContext::default();
                    let mut block = VectorSortBlock::<Matrix<3, 3, $type>, Matrix<1, 9, $type>>::default();
                    let parameters = Parameters::new("Descending");

                    // This test has redundant values (3 and 7)
                    let input = Matrix {
                        data: [[
                                [<3 $type>], [<7 $type>], [<9 $type>]],
                                [[<7 $type>], [<5 $type>], [<2 $type>]],
                                [[<1 $type>], [<10 $type>], [<3 $type>]
                            ]],
                    };

                    let output = block.process(&parameters, &c, &input).data;

                    assert_eq!(
                        output,
                        [
                            [[<10 $type>]],
                            [[<9 $type>]],
                            [[<7 $type>]],
                            [[<7 $type>]],
                            [[<5 $type>]],
                            [[<3 $type>]],
                            [[<3 $type>]],
                            [[<2 $type>]],
                            [[<1 $type>]]
                        ]
                    );
                }
            }
        };
    }

    impl_vector_sort_tests!(f64);
    impl_vector_sort_tests!(f32);
    impl_vector_sort_tests!(i32);
    impl_vector_sort_tests!(u32);
    impl_vector_sort_tests!(i16);
    impl_vector_sort_tests!(u16);
    impl_vector_sort_tests!(i8);
    impl_vector_sort_tests!(u8);
}
