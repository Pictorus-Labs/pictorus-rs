use block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock, Scalar};
use strum::EnumString;

/// This Block detects change in it input and emits True if change is
/// detected, False otherwise. It only accepts a single input edge.
/// The edge can optionally be a Matrix, in that case the change detection
/// operation is performed element wise and the output is a Matrix of the
/// same size as the input.
///
/// The block can detect 3 different "modes" of change:
///  - Rising: Only triggers if the input value got larger
///  - Falling: Only triggers if the input value got smaller
///  - Any: Triggers if the input value changed at all
///
/// The first execution of this block has no value to compare with
/// so it will always emit `false`
pub struct ChangeDetectionBlock<T: Apply> {
    pub data: OldBlockData,
    buffer: Option<T::Output>,
    last_input: Option<T>,
}

impl<T> Default for ChangeDetectionBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            buffer: None,
            last_input: None,
        }
    }
}

impl<T> HasIc for ChangeDetectionBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        ChangeDetectionBlock::<T> {
            buffer: Some(T::Output::default()),
            data: <OldBlockData as FromPass<T::Output>>::from_pass(T::Output::default().as_by()),
            last_input: Some(parameters.ic),
        }
    }
}

impl<T> ProcessBlock for ChangeDetectionBlock<T>
where
    T: Apply,
    OldBlockData: FromPass<T::Output>,
{
    type Inputs = T;
    type Output = T::Output;
    type Parameters = Parameters<T>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: PassBy<'_, Self::Inputs>,
    ) -> PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs, parameters, &mut self.last_input);
        self.data = OldBlockData::from_pass(output);
        output
    }
}

pub trait Apply: Pass + Sized + Copy {
    type Output: Pass + Default;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        params: &Parameters<Self>,
        last_input: &mut Option<Self>,
    ) -> PassBy<'s, Self::Output>;
}

impl<C: ChangeDetect> Apply for C {
    type Output = bool;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        params: &Parameters<Self>,
        last_input: &mut Option<Self>,
    ) -> PassBy<'s, Self::Output> {
        // Store a copy of input in `last_input` while grabbing what was there as `old_last_input`
        let old_last_input = last_input.replace(input);
        let old_last_input = old_last_input.unwrap_or(params.ic);
        let res = Self::change_detect(input, old_last_input, params.change_mode);
        *store = Some(res);
        res.as_by()
    }
}

impl<const NROWS: usize, const NCOLS: usize, C: ChangeDetect> Apply for Matrix<NROWS, NCOLS, C> {
    type Output = Matrix<NROWS, NCOLS, bool>;

    fn apply<'s>(
        store: &'s mut Option<Self::Output>,
        input: PassBy<Self>,
        params: &Parameters<Self>,
        last_input: &mut Option<Self>,
    ) -> PassBy<'s, Self::Output> {
        // Store a copy of input in `last_input` while grabbing what was there as `old_last_input`
        let old_last_input = last_input.replace(*input);
        let old_last_input = old_last_input.unwrap_or(params.ic);
        // Initialize Output to a Zeroed (i.e. all `false`) state.
        let output = store.insert(Matrix::zeroed());
        // Make a immutable iterator of each element of `input` and `old_last_input`

        let inputs = input
            .data
            .as_flattened()
            .iter()
            .zip(old_last_input.data.as_flattened().iter());
        // Zip that iterator with a mutable iterator over the output matrix
        // and then perform the operation on each set of three values
        output
            .data
            .as_flattened_mut()
            .iter_mut()
            .zip(inputs)
            .for_each(|(output, (lh, rh))| {
                *output = C::change_detect(*lh, *rh, params.change_mode)
            });
        output
    }
}

trait ChangeDetect: Scalar + for<'a> Pass<By<'a> = Self> + PartialEq + PartialOrd {
    fn change_detect(left_hand: PassBy<Self>, right_hand: PassBy<Self>, mode: ChangeMode) -> bool {
        match mode {
            ChangeMode::Any => left_hand != right_hand,
            ChangeMode::Rising => left_hand > right_hand,
            ChangeMode::Falling => left_hand < right_hand,
        }
    }
}

impl ChangeDetect for u8 {}
impl ChangeDetect for i8 {}
impl ChangeDetect for u16 {}
impl ChangeDetect for i16 {}
impl ChangeDetect for u32 {}
impl ChangeDetect for i32 {}
impl ChangeDetect for f32 {}
impl ChangeDetect for f64 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
pub enum ChangeMode {
    Any,
    Rising,
    Falling,
}

pub struct Parameters<T> {
    ic: T,
    change_mode: ChangeMode,
}

impl<T> Parameters<T> {
    pub fn new(ic: T, change_mode: &str) -> Self {
        let change_mode = change_mode.parse().expect("Failed to parse ChangeMode");
        Self { ic, change_mode }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use paste::paste;

    macro_rules! test_scalars {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_scalar_rising_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new([<1 $type>], "Rising");
                    let mut block = ChangeDetectionBlock::<$type>::default();

                    // No change - false
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(!output);

                    //Falling -false
                    let output = block.process(&params, &context, [<0 $type>]);
                    assert!(!output);

                    // Rising - true
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(output);
                }

                #[test]
                fn [<test_scalar_falling_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new([<1 $type>], "Falling");
                    let mut block = ChangeDetectionBlock::<$type>::default();

                    // No change - false
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(!output);

                    //Falling -true
                    let output = block.process(&params, &context, [<0 $type>]);
                    assert!(output);

                    // Rising - false
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(!output);
                }


                #[test]
                fn [<test_scalar_any_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new([<1 $type>], "Any");
                    let mut block = ChangeDetectionBlock::<$type>::default();

                    // No change - false
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(!output);

                    //Falling -true
                    let output = block.process(&params, &context, [<0 $type>]);
                    assert!(output);

                    // Rising - true
                    let output = block.process(&params, &context, [<1 $type>]);
                    assert!(output);
                }
            }
        };
    }
    test_scalars!(u8);
    test_scalars!(i8);
    test_scalars!(u16);
    test_scalars!(i16);
    test_scalars!(u32);
    test_scalars!(i32);
    test_scalars!(f32);
    test_scalars!(f64);

    macro_rules! test_matrix {
        ($type:ty) => {
            paste! {
                #[test]
                fn [<test_matrix_falling_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new(Matrix{data: [[[<42 $type>]; 8]; 11],}, "Falling");
                    let mut block = ChangeDetectionBlock::<Matrix<8, 11, $type>>::default();

                    let input = Matrix {
                        data: [[[<42 $type>]; 8]; 11],
                    };

                    // No change
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    // Falling for all values
                    let input = Matrix {
                        data: [[[<1 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[true; 8]; 11]
                        }
                    );

                    //Rising all values
                    let mut input = Matrix {
                        data: [[[<11 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    // Falling just one element
                    input.data[3][5] = [<4 $type>];
                    let mut expected_output = Matrix::zeroed();
                    expected_output.data[3][5] = true;
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &expected_output);

                    // Rising just one element
                    input.data[6][2] = [<42 $type>];
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());
                }

                #[test]
                fn [<test_matrix_rising_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new(Matrix{data: [[[<42 $type>]; 8]; 11],}, "Rising");
                    let mut block = ChangeDetectionBlock::<Matrix<8, 11, $type>>::default();

                    let input = Matrix {
                        data: [[[<42 $type>]; 8]; 11],
                    };

                    // No change
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    // Falling for all values
                    let input = Matrix {
                        data: [[[<1 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    //Rising all values
                    let mut input = Matrix {
                        data: [[[<11 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[true; 8]; 11]
                        }
                    );

                    // Falling just one element
                    input.data[3][5] = [<4 $type>];
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    // Rising just one element
                    input.data[6][2] = [<42 $type>];
                    let mut expected_output = Matrix::zeroed();
                    expected_output.data[6][2] = true;
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &expected_output);
                }

                #[test]
                fn [<test_matrix_any_ $type>]() {
                    let context = StubContext::default();
                    let params = Parameters::new(Matrix{data: [[[<42 $type>]; 8]; 11],}, "Any");
                    let mut block = ChangeDetectionBlock::<Matrix<8, 11, $type>>::default();

                    let input = Matrix {
                        data: [[[<42 $type>]; 8]; 11],
                    };

                    // No change
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &Matrix::zeroed());

                    // Falling for all values
                    let input = Matrix {
                        data: [[[<1 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[true; 8]; 11]
                        }
                    );

                    //Rising all values
                    let mut input = Matrix {
                        data: [[[<11 $type>]; 8]; 11],
                    };
                    let output = block.process(&params, &context, &input);
                    assert_eq!(
                        output,
                        &Matrix {
                            data: [[true; 8]; 11]
                        }
                    );

                    // Falling just one element
                    input.data[3][5] = [<4 $type>];
                    let mut expected_output = Matrix::zeroed();
                    expected_output.data[3][5] = true;
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &expected_output);

                    // Rising just one element
                    input.data[6][2] = [<42 $type>];
                    let mut expected_output = Matrix::zeroed();
                    expected_output.data[6][2] = true;
                    let output = block.process(&params, &context, &input);
                    assert_eq!(output, &expected_output);
                }
            }
        };
    }

    test_matrix!(u8);
    test_matrix!(i8);
    test_matrix!(u16);
    test_matrix!(i16);
    test_matrix!(u32);
    test_matrix!(i32);
    test_matrix!(f32);
    test_matrix!(f64);
}
