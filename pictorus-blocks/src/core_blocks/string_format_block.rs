use crate::traits::{Scalar, Serialize};
use alloc::{string::String, vec::Vec};
use pictorus_traits::{ByteSliceSignal, Matrix, Pass, PassBy, ProcessBlock};

pub struct Parameters<const N: usize> {
    pub formatter_function: fn(&[&str; N]) -> String,
}

impl<const N: usize> Parameters<N> {
    pub fn new(formatter_function: fn(&[&str; N]) -> String) -> Self {
        Self { formatter_function }
    }
}

/// Formats the input data into a string using the provided formatter function.
///
/// If a ByteSliceSignal is provided, it will be converted to a UTF-8 string.
/// Other data types are serialized to their equivalent JSON strings.
/// For numbers this is just the number as a string. For matrices, this will
/// be the matrix data in row-major order.
pub struct StringFormatBlock<T: Apply> {
    phantom_output_type: core::marker::PhantomData<T>,
    pub data: Vec<u8>,
}

impl<T: Apply> Default for StringFormatBlock<T> {
    fn default() -> Self {
        Self {
            phantom_output_type: core::marker::PhantomData,
            data: Vec::new(),
        }
    }
}

impl<T> ProcessBlock for StringFormatBlock<T>
where
    T: Apply,
{
    type Output = ByteSliceSignal;
    type Inputs = T;
    type Parameters = T::Parameters;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let formatted_string = T::apply(inputs, parameters);
        self.data = formatted_string.as_bytes().to_vec();
        &self.data
    }
}

pub trait ToString: Pass {
    fn to_string(input: PassBy<Self>) -> String;
}

impl<T: Scalar + Serialize<FormatOptions = ()>> ToString for T {
    fn to_string(input: PassBy<Self>) -> String {
        miniserde::json::to_string(&T::as_json_value(input, ()))
    }
}

impl<T: Scalar, const NROWS: usize, const NCOLS: usize> ToString for Matrix<NROWS, NCOLS, T>
where
    Matrix<NROWS, NCOLS, T>: Serialize<FormatOptions = ()> + for<'a> Pass<By<'a> = &'a Self>,
{
    fn to_string(input: PassBy<Self>) -> String {
        miniserde::json::to_string(&Matrix::as_json_value(input, ()))
    }
}

impl ToString for ByteSliceSignal {
    fn to_string(input: PassBy<Self>) -> String {
        String::from_utf8(input.to_vec()).unwrap_or_default()
    }
}

pub trait Apply: Pass {
    type Parameters;
    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String;
}

impl<T: ToString> Apply for T {
    type Parameters = Parameters<1>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[&T::to_string(input)])
    }
}

impl<T1: ToString, T2: ToString> Apply for (T1, T2) {
    type Parameters = Parameters<2>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[&T1::to_string(input.0), &T2::to_string(input.1)])
    }
}
impl<T1: ToString, T2: ToString, T3: ToString> Apply for (T1, T2, T3) {
    type Parameters = Parameters<3>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
        ])
    }
}
impl<T1: ToString, T2: ToString, T3: ToString, T4: ToString> Apply for (T1, T2, T3, T4) {
    type Parameters = Parameters<4>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
            &T4::to_string(input.3),
        ])
    }
}
impl<T1: ToString, T2: ToString, T3: ToString, T4: ToString, T5: ToString> Apply
    for (T1, T2, T3, T4, T5)
{
    type Parameters = Parameters<5>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
            &T4::to_string(input.3),
            &T5::to_string(input.4),
        ])
    }
}
impl<T1: ToString, T2: ToString, T3: ToString, T4: ToString, T5: ToString, T6: ToString> Apply
    for (T1, T2, T3, T4, T5, T6)
{
    type Parameters = Parameters<6>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
            &T4::to_string(input.3),
            &T5::to_string(input.4),
            &T6::to_string(input.5),
        ])
    }
}
impl<
        T1: ToString,
        T2: ToString,
        T3: ToString,
        T4: ToString,
        T5: ToString,
        T6: ToString,
        T7: ToString,
    > Apply for (T1, T2, T3, T4, T5, T6, T7)
{
    type Parameters = Parameters<7>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
            &T4::to_string(input.3),
            &T5::to_string(input.4),
            &T6::to_string(input.5),
            &T7::to_string(input.6),
        ])
    }
}
impl<
        T1: ToString,
        T2: ToString,
        T3: ToString,
        T4: ToString,
        T5: ToString,
        T6: ToString,
        T7: ToString,
        T8: ToString,
    > Apply for (T1, T2, T3, T4, T5, T6, T7, T8)
{
    type Parameters = Parameters<8>;

    fn apply(input: PassBy<Self>, parameters: &Self::Parameters) -> String {
        (parameters.formatter_function)(&[
            &T1::to_string(input.0),
            &T2::to_string(input.1),
            &T3::to_string(input.2),
            &T4::to_string(input.3),
            &T5::to_string(input.4),
            &T6::to_string(input.5),
            &T7::to_string(input.6),
            &T8::to_string(input.7),
        ])
    }
}

#[macro_export]
macro_rules! build_string_format_closure {
    ($format_str:expr, 1) => {
        |inputs: &[&str; 1]| alloc::format!($format_str, inputs[0])
    };
    ($format_str:expr, 2) => {
        |inputs: &[&str; 2]| alloc::format!($format_str, inputs[0], inputs[1])
    };
    ($format_str:expr, 3) => {
        |inputs: &[&str; 3]| alloc::format!($format_str, inputs[0], inputs[1], inputs[2])
    };
    ($format_str:expr, 4) => {
        |inputs: &[&str; 4]| alloc::format!($format_str, inputs[0], inputs[1], inputs[2], inputs[3])
    };
    ($format_str:expr, 5) => {
        |inputs: &[&str; 5]| {
            alloc::format!(
                $format_str,
                inputs[0],
                inputs[1],
                inputs[2],
                inputs[3],
                inputs[4]
            )
        }
    };
    ($format_str:expr, 6) => {
        |inputs: &[&str; 6]| {
            alloc::format!(
                $format_str,
                inputs[0],
                inputs[1],
                inputs[2],
                inputs[3],
                inputs[4],
                inputs[5]
            )
        }
    };
    ($format_str:expr, 7) => {
        |inputs: &[&str; 7]| {
            alloc::format!(
                $format_str,
                inputs[0],
                inputs[1],
                inputs[2],
                inputs[3],
                inputs[4],
                inputs[5],
                inputs[6]
            )
        }
    };
    ($format_str:expr, 8) => {
        |inputs: &[&str; 8]| {
            alloc::format!(
                $format_str,
                inputs[0],
                inputs[1],
                inputs[2],
                inputs[3],
                inputs[4],
                inputs[5],
                inputs[6],
                inputs[7]
            )
        }
    };
    ($format_str:expr, $size:expr) => {
        compile_error!("Unsupported size for formatter function")
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use alloc::format;

    #[test]
    fn test_string_format_block() {
        let formatter = |inputs: &[&str; 3]| format!("{} {} {}", inputs[0], inputs[1], inputs[2]);
        let parameters = Parameters::new(formatter);
        let mut block = StringFormatBlock::<(f64, ByteSliceSignal, Matrix<2, 2, f64>)>::default();

        let input = (42.0, "Foo".as_bytes(), &Matrix::zeroed());
        let context = StubContext::default();
        let output = block.process(&parameters, &context, input);

        assert_eq!(
            std::str::from_utf8(output).unwrap(),
            "42.0 Foo [[0.0,0.0],[0.0,0.0]]"
        );
    }

    #[test]
    fn test_string_format_block_matrix() {
        let formatter = |inputs: &[&str; 1]| format!("Matrix: {}", inputs[0]);
        let parameters = Parameters::new(formatter);
        let mut block = StringFormatBlock::<Matrix<2, 2, f64>>::default();

        let input = Matrix {
            // Input data is col-major. We expect the output to be row-major.
            data: [[1.0, 3.0], [2.0, 4.0]],
        };
        let context = StubContext::default();
        let output = block.process(&parameters, &context, &input);

        assert_eq!(
            std::str::from_utf8(output).unwrap(),
            "Matrix: [[1.0,2.0],[3.0,4.0]]"
        );
    }

    #[test]
    fn test_params_macro() {
        let formatter = |inputs: &[&str; 3]| format!("{} {} {}", inputs[0], inputs[1], inputs[2]);
        let parameters = Parameters::new(formatter);
        let test_data = ["foo", "bar", "baz"];
        assert_eq!(
            (parameters.formatter_function)(&test_data),
            (build_string_format_closure!("{} {} {}", 3))(&test_data)
        );
    }
}
