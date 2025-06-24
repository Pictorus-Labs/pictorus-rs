use pictorus_block_data::{BlockData as OldBlockData, FromPass};
#[cfg(feature = "tricore")]
use pictorus_traits::FlattenSlice as _;
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

#[derive(strum::EnumString, Clone, Copy)]
pub enum NotMethod {
    Logical,
    Bitwise,
}

/// A block that performs a logical or bitwise NOT operation on the input.
pub struct NotBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    pub data: OldBlockData,
    buffer: Option<T::Output>,
}

impl<T> Default for NotBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
        }
    }
}

impl<T> ProcessBlock for NotBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters.method);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        method: NotMethod,
    ) -> PassBy<'s, Self::Output>;
}

impl Apply for bool {
    type Output = bool;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        method: NotMethod,
    ) -> PassBy<'s, Self::Output> {
        let output = match method {
            NotMethod::Logical => !input,
            NotMethod::Bitwise => !input,
        };
        *store = Some(output);
        output
    }
}

impl<const NROWS: usize, const NCOLS: usize> Apply for Matrix<NROWS, NCOLS, bool> {
    type Output = Matrix<NROWS, NCOLS, bool>;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        method: NotMethod,
    ) -> PassBy<'s, Self::Output> {
        let output = store.insert(Matrix::zeroed());
        output
            .data
            .as_flattened_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(i, lhs)| {
                let input_val = input.data.as_flattened()[i];
                *lhs = match method {
                    NotMethod::Logical => !input_val,
                    NotMethod::Bitwise => !input_val,
                };
            });
        output
    }
}

macro_rules! impl_not_apply {
    ($type:ty, $cast_type:ty) => {
        impl Apply for $type {
            type Output = $type;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
                method: NotMethod,
            ) -> PassBy<'s, Self::Output> {
                let output = match method {
                    NotMethod::Logical => {
                        if input == 0.0 {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    NotMethod::Bitwise => !(input as $cast_type) as $type,
                };
                *store = Some(output);
                output
            }
        }

        impl<const NROWS: usize, const NCOLS: usize> Apply for Matrix<NROWS, NCOLS, $type> {
            type Output = Matrix<NROWS, NCOLS, $type>;

            fn apply<'s>(
                store: &'s mut Option<Self::Output>,
                input: PassBy<Self>,
                method: NotMethod,
            ) -> PassBy<'s, Self::Output> {
                let output = store.insert(Matrix::zeroed());
                output
                    .data
                    .as_flattened_mut()
                    .iter_mut()
                    .enumerate()
                    .for_each(|(i, lhs)| {
                        let input_val = input.data.as_flattened()[i];
                        *lhs = match method {
                            NotMethod::Logical => {
                                if input_val == 0.0 {
                                    1.0
                                } else {
                                    0.0
                                }
                            }
                            NotMethod::Bitwise => !(input_val as $cast_type) as $type,
                        };
                    });
                output
            }
        }
    };
}

pub struct Parameters {
    // The method to use for the NOT operation. Either 'Logical' or 'Bitwise'.
    pub method: NotMethod,
}

impl Parameters {
    pub fn new(method: &str) -> Self {
        Self {
            method: method
                .parse()
                .expect("Failed to parse NotMethod, expected 'Logical' or 'Bitwise'"),
        }
    }
}

impl_not_apply!(f32, i32);
impl_not_apply!(f64, i64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_not_block {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_not_block_logical_scalar_ $type>]() {
                    let mut block = NotBlock::<$type>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Logical");

                    let res = block.process(&parameters, &context, 1.0);
                    assert_eq!(res, 0.0);
                    assert_eq!(block.data.scalar(), 0.0);

                    let res = block.process(&parameters, &context, 0.0);
                    assert_eq!(res, 1.0);
                    assert_eq!(block.data.scalar(), 1.0);

                    let res = block.process(&parameters, &context, -1.2);
                    assert_eq!(res, 0.0);
                    assert_eq!(block.data.scalar(), 0.0);

                    let res = block.process(&parameters, &context, 1.2);
                    assert_eq!(res, 0.0);
                    assert_eq!(block.data.scalar(), 0.0);
                }

                #[test]
                fn [<test_not_block_logical_matrix_ $type>]() {
                    let mut block = NotBlock::<Matrix<4, 1, $type>>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Logical");

                    let input = Matrix {
                        data: [[1.0, 0.0, -1.2, 1.2]],
                    };
                    let res = block.process(&parameters, &context, &input);
                    assert_eq!(res.data, [[0.0, 1.0, 0.0, 0.0]]);
                    assert_eq!(block.data.get_data().as_slice(), [[0.0, 1.0, 0.0, 0.0]].as_flattened());
                }

                #[test]
                fn [<test_not_block_bitwise_scalar_ $type>]() {
                    let mut block = NotBlock::<$type>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Bitwise");

                    let res = block.process(&parameters, &context, 1.0);
                    assert_eq!(res, -2.0);
                    assert_eq!(block.data.scalar(), -2.0);

                    let res = block.process(&parameters, &context, 42.0);
                    assert_eq!(res, -43.0);
                    assert_eq!(block.data.scalar(), -43.0);

                    let res = block.process(&parameters, &context, -1.2);
                    assert_eq!(res, 0.0);
                    assert_eq!(block.data.scalar(), 0.0);

                    let res = block.process(&parameters, &context, 1.2);
                    assert_eq!(res, -2.0);
                    assert_eq!(block.data.scalar(), -2.0);
                }

                #[test]
                fn [<test_not_block_bitwise_matrix_ $type>]() {
                    let mut block = NotBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Bitwise");

                    let input = Matrix {
                        data: [[1.0, 42.0], [-1.2, 1.2]],
                    };
                    let res = block.process(&parameters, &context, &input);
                    assert_eq!(res.data, [[-2.0, -43.0], [0.0, -2.0]]);
                    assert_eq!(block.data.get_data().as_slice(), [[-2.0, -43.0], [0.0, -2.0]].as_flattened());
                }
            }
        };
    }

    test_not_block!(f32);
    test_not_block!(f64);

    #[test]
    fn test_scalar_bool() {
        let mut block = NotBlock::<bool>::default();
        let context = StubContext::default();
        let parameters = Parameters::new("Logical");

        let res = block.process(&parameters, &context, true);
        assert!(!res);
        assert_eq!(block.data.scalar(), 0.0);

        let res = block.process(&parameters, &context, false);
        assert!(res);
        assert_eq!(block.data.scalar(), 1.0);

        let parameters = Parameters::new("Bitwise");
        let res = block.process(&parameters, &context, true);
        assert!(!res);
        assert_eq!(block.data.scalar(), 0.0);

        let res = block.process(&parameters, &context, false);
        assert!(res);
        assert_eq!(block.data.scalar(), 1.0);
    }

    #[test]
    fn test_matrix_bool() {
        let mut block = NotBlock::<Matrix<2, 2, bool>>::default();
        let context = StubContext::default();
        let parameters = Parameters::new("Logical");

        let input = Matrix {
            data: [[true, false], [false, true]],
        };
        let res = block.process(&parameters, &context, &input);
        assert_eq!(res.data, [[false, true], [true, false]]);
        assert_eq!(
            block.data.get_data().as_slice(),
            [[0.0, 1.0], [1.0, 0.0]].as_flattened()
        );
    }
}
