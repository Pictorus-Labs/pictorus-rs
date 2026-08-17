use crate::traits::Scalar;
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
{
    buffer: T,
}

impl<T> Default for NotBlock<T>
where
    T: Apply,
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
        }
    }
}

impl<T> ProcessBlock for NotBlock<T>
where
    T: Apply,
{
    type Inputs = T;
    type Output = T;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let output = T::apply(&mut self.buffer, input, parameters.method);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

pub trait Apply: Pass + Default {
    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, method: NotMethod) -> PassBy<'s, Self>;
}

impl Apply for bool {
    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, method: NotMethod) -> PassBy<'s, Self> {
        let output = match method {
            NotMethod::Logical => !input,
            NotMethod::Bitwise => !input,
        };
        *store = output;
        output
    }
}

macro_rules! impl_not_apply {
    ($type:ty) => {
        impl Apply for $type {
            fn apply<'s>(
                store: &'s mut Self,
                input: PassBy<Self>,
                method: NotMethod,
            ) -> PassBy<'s, Self> {
                let output = match method {
                    NotMethod::Logical => Self::from_bool(!input.is_truthy()),
                    NotMethod::Bitwise => !input,
                };
                *store = output;
                output
            }
        }
    };
    ($type:ty, $cast_type:ty) => {
        impl Apply for $type {
            fn apply<'s>(
                store: &'s mut Self,
                input: PassBy<Self>,
                method: NotMethod,
            ) -> PassBy<'s, Self> {
                let output = match method {
                    NotMethod::Logical => Self::from_bool(!input.is_truthy()),
                    NotMethod::Bitwise => !(input as $cast_type) as $type,
                };
                *store = output;
                output
            }
        }
    };
}

impl<T: Apply + Pass + Scalar, const NROWS: usize, const NCOLS: usize> Apply
    for Matrix<NROWS, NCOLS, T>
where
    for<'a> Matrix<NROWS, NCOLS, T>: Pass<By<'a> = &'a Self>,
{
    fn apply<'s>(store: &'s mut Self, input: PassBy<Self>, method: NotMethod) -> PassBy<'s, Self> {
        *store = Matrix::zeroed();
        store
            .data
            .as_flattened_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(i, lhs)| {
                let input_val = input.data.as_flattened()[i];
                T::apply(lhs, input_val, method);
            });
        store
    }
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
impl_not_apply!(i32);
impl_not_apply!(i64);
impl_not_apply!(u32);
impl_not_apply!(u64);
impl_not_apply!(u8);
impl_not_apply!(u16);
impl_not_apply!(i8);
impl_not_apply!(i16);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use num_traits::{One, Zero};
    use paste::paste;

    #[test]
    fn test_not_default_buffer_no_panic() {
        let block = NotBlock::<f64>::default();
        assert_eq!(block.buffer(), 0.0);

        let block = NotBlock::<bool>::default();
        assert!(!block.buffer());
    }

    macro_rules! test_not_block {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_not_block_logical_scalar_ $type>]() {
                    let mut block = NotBlock::<$type>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Logical");

                    let res = block.process(&parameters, &context, $type::one());
                    assert_eq!(res, $type::zero());
                    assert_eq!(block.buffer(), res);

                    let res = block.process(&parameters, &context, $type::zero());
                    assert_eq!(res, $type::one());
                    assert_eq!(block.buffer(), $type::one());
                }

                #[test]
                fn [<test_not_block_logical_matrix_ $type>]() {
                    let mut block = NotBlock::<Matrix<4, 1, $type>>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Logical");

                    let input = Matrix {
                        data: [[$type::one(), $type::zero(), $type::one(), $type::one()]],
                    };
                    let res = block.process(&parameters, &context, &input);
                    assert_eq!(res.data, [[$type::zero(), $type::one(), $type::zero(), $type::zero()]]);
                    assert_eq!(block.buffer().data, [[$type::zero(), $type::one(), $type::zero(), $type::zero()]]);
                }

                #[test]
                fn [<test_not_block_bitwise_scalar_ $type>]() {
                    let mut block = NotBlock::<$type>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Bitwise");

                    let res = block.process(&parameters, &context, 0b1 as $type);
                    assert_eq!(res, !0b1 as $type);
                    assert_eq!(block.buffer(), !0b1 as $type);

                    let res = block.process(&parameters, &context, 42 as $type);
                    assert_eq!(res, !42 as $type);
                    assert_eq!(block.buffer(), !42 as $type);

                    let res = block.process(&parameters, &context, -1i8 as $type);
                    assert_eq!(res, !-1i8 as $type);
                    assert_eq!(block.buffer(), !-1i8 as $type);

                    let res = block.process(&parameters, &context, 1 as $type);
                    assert_eq!(res, !1 as $type);
                    assert_eq!(block.buffer(), !1 as $type);
                }

                #[test]
                fn [<test_not_block_bitwise_matrix_ $type>]() {
                    let mut block = NotBlock::<Matrix<2, 2, $type>>::default();
                    let context = StubContext::default();
                    let parameters = Parameters::new("Bitwise");

                    let input = Matrix {
                        data: [[1 as $type, 42 as $type], [-1i8 as $type, 1 as $type]],
                    };
                    let res = block.process(&parameters, &context, &input);
                    assert_eq!(res.data, [[!1 as $type, !42 as $type], [!-1i8 as $type, !1 as $type]]);
                    assert_eq!(block.buffer().data, [[!1 as $type, !42 as $type], [!-1i8 as $type, !1 as $type]]);
                }
            }
        };
    }

    test_not_block!(f32);
    test_not_block!(f64);
    test_not_block!(i64);
    test_not_block!(u64);
    test_not_block!(i8);
    test_not_block!(i16);
    test_not_block!(i32);
    test_not_block!(u8);
    test_not_block!(u16);
    test_not_block!(u32);

    #[test]
    fn test_scalar_bool() {
        let mut block = NotBlock::<bool>::default();
        let context = StubContext::default();
        let parameters = Parameters::new("Logical");

        let res = block.process(&parameters, &context, true);
        assert!(!res);
        assert!(!block.buffer());

        let res = block.process(&parameters, &context, false);
        assert!(res);
        assert!(block.buffer());

        let parameters = Parameters::new("Bitwise");
        let res = block.process(&parameters, &context, true);
        assert!(!res);
        assert!(!block.buffer());

        let res = block.process(&parameters, &context, false);
        assert!(res);
        assert!(block.buffer());
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
        assert_eq!(block.buffer().data, [[false, true], [true, false]]);
    }
}
