use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Scalar;

/// Parameters for VectorMergeBlock
#[derive(Clone, Copy)]
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

/// Merges vectors and scalars into a single vector.
///
/// Vectors are flattened before merging, and the output is a 1xN matrix where N is the sum of the rows*cols of
/// the inputs. Currently implemented for up to 8 `Scalar` or `Matrix` inputs.
///
/// Generic arguments:
/// - \<O\>: The output type of the block
/// - \<I\>: A tuple of inputs that will be merged into the output
pub struct VectorMergeBlock<O, I>
where
    I: Pass + Mergeable<O>,
{
    pub data: OldBlockData,
    buffer: Option<<I as Mergeable<O>>::Output>,
    _phantom: core::marker::PhantomData<I>,
}

impl<O, I> Default for VectorMergeBlock<O, I>
where
    I: Pass + Mergeable<O>,
    O: Pass + Default,
    OldBlockData: FromPass<<I as Mergeable<O>>::Output>,
{
    fn default() -> Self {
        Self {
            data: <OldBlockData as FromPass<<I as Mergeable<O>>::Output>>::from_pass(
                <I as Mergeable<O>>::Output::default().as_by(),
            ),
            buffer: None,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<O, I> ProcessBlock for VectorMergeBlock<O, I>
where
    I: Pass + Mergeable<O>,
    O: Pass + Default,
    OldBlockData: FromPass<<I as Mergeable<O>>::Output>,
{
    type Inputs = I;
    type Output = <I as Mergeable<O>>::Output;
    type Parameters = Parameters;

    fn process(
        &mut self,
        _parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        let mut offset = 0;
        let output = I::get_merge(input, &mut offset, &mut self.buffer);
        self.data = OldBlockData::from_pass(output);
        self.data
            .set_type(pictorus_block_data::BlockDataType::Vector);
        output
    }
}

pub trait MergeInto<DEST: Pass>: Pass {
    fn merge_into<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<DEST>,
    ) -> PassBy<'a, DEST>;
}

/// Merging a matrix
impl<const IROW: usize, const ICOL: usize, const OCOL: usize, S: Scalar>
    MergeInto<Matrix<1, OCOL, S>> for Matrix<IROW, ICOL, S>
{
    fn merge_into<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Matrix<1, OCOL, S>>,
    ) -> PassBy<'a, Matrix<1, OCOL, S>> {
        let dest = dest.get_or_insert(Matrix::<1, OCOL, S>::zeroed());
        dest.data.as_flattened_mut()[*offset..(*offset + input.data.as_flattened().len())]
            .copy_from_slice(input.data.as_flattened());
        *offset += input.data.as_flattened().len();
        dest
    }
}

/// Merging a scalar
impl<const OCOL: usize, S: Scalar> MergeInto<Matrix<1, OCOL, S>> for S {
    fn merge_into<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Matrix<1, OCOL, S>>,
    ) -> PassBy<'a, Matrix<1, OCOL, S>> {
        let dest = dest.get_or_insert(Matrix::<1, OCOL, S>::zeroed());
        dest.data.as_flattened_mut()[*offset] = input;
        *offset += 1;
        dest
    }
}

pub trait Mergeable<O>: Pass {
    type Output: Pass + Default;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output>;
}

/// Merging a Matrix tuple element
impl<const IROW: usize, const ICOL: usize, const OCOL: usize, S: Scalar>
    Mergeable<Matrix<1, OCOL, S>> for Matrix<IROW, ICOL, S>
{
    type Output = Matrix<1, OCOL, S>;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        Self::merge_into(input, offset, dest);
        dest.as_ref().unwrap()
    }
}

/// Merging a scalar tuple element
impl<O, A> Mergeable<O> for A
where
    A: Scalar,
    A: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let a = input;
        A::merge_into(a, offset, dest)
    }
}

impl<O, A, B> Mergeable<O> for (A, B)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest)
    }
}

impl<O, A, B, C> Mergeable<O> for (A, B, C)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest)
    }
}

impl<O, A, B, C, D> Mergeable<O> for (A, B, C, D)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    D: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest);
        D::merge_into(d, offset, dest)
    }
}

impl<O, A, B, C, D, E> Mergeable<O> for (A, B, C, D, E)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    D: MergeInto<O>,
    E: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest);
        D::merge_into(d, offset, dest);
        E::merge_into(e, offset, dest)
    }
}

impl<O, A, B, C, D, E, F> Mergeable<O> for (A, B, C, D, E, F)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    D: MergeInto<O>,
    E: MergeInto<O>,
    F: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest);
        D::merge_into(d, offset, dest);
        E::merge_into(e, offset, dest);
        F::merge_into(f, offset, dest)
    }
}

impl<O, A, B, C, D, E, F, G> Mergeable<O> for (A, B, C, D, E, F, G)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    D: MergeInto<O>,
    E: MergeInto<O>,
    F: MergeInto<O>,
    G: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest);
        D::merge_into(d, offset, dest);
        E::merge_into(e, offset, dest);
        F::merge_into(f, offset, dest);
        G::merge_into(g, offset, dest)
    }
}

impl<O, A, B, C, D, E, F, G, H> Mergeable<O> for (A, B, C, D, E, F, G, H)
where
    A: MergeInto<O>,
    B: MergeInto<O>,
    C: MergeInto<O>,
    D: MergeInto<O>,
    E: MergeInto<O>,
    F: MergeInto<O>,
    G: MergeInto<O>,
    H: MergeInto<O>,
    O: Pass + Default,
{
    type Output = O;

    fn get_merge<'a>(
        input: PassBy<Self>,
        offset: &mut usize,
        dest: &'a mut Option<Self::Output>,
    ) -> PassBy<'a, Self::Output> {
        let (a, b, c, d, e, f, g, h) = input;
        A::merge_into(a, offset, dest);
        B::merge_into(b, offset, dest);
        C::merge_into(c, offset, dest);
        D::merge_into(d, offset, dest);
        E::merge_into(e, offset, dest);
        F::merge_into(f, offset, dest);
        G::merge_into(g, offset, dest);
        H::merge_into(h, offset, dest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use pictorus_block_data::{BlockData as OldBlockData, BlockDataType, ToPass};

    #[test]
    fn test_vector_merge_block_scalar_original_test() {
        // Should be able to pass in scalars, vectors, or matrices,
        // and get back a flattened vector
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let mut block = VectorMergeBlock::<
            Matrix<1, 8, f64>,
            (f64, Matrix<1, 3, f64>, Matrix<2, 2, f64>),
        >::default();
        let input_m = Matrix {
            data: [[5., 6.], [7., 8.]],
        };
        let input_v = Matrix {
            data: [[2.], [3.], [4.]],
        };

        let signal1 = OldBlockData::from_scalar(1.0);
        let signal2 = OldBlockData::from_vector(&[2.0, 3.0, 4.0]);
        let signal3 = OldBlockData::new(2, 2, &[5.0, 6.0, 7.0, 8.0]);

        let _output = block.process(
            &parameters,
            &stub_context,
            (signal1.to_pass(), &signal2.to_pass(), &signal3.to_pass()),
        );
        assert_eq!(block.data.get_type(), BlockDataType::Vector);

        // Note, the original test uses row-major order and Corelib uses column-major order, so 6 and 7 are swapped
        // let original_expected = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let expected = [1.0, 2.0, 3.0, 4.0, 5.0, 7.0, 6.0, 8.0];
        assert!(block.data.vector().iter().eq(expected.iter()));

        let output = block.process(&parameters, &stub_context, (1.0, &input_v, &input_m));
        assert_eq!(
            output.data.as_flattened(),
            &[1., 2., 3., 4., 5., 6., 7., 8.]
        );
    }

    #[test]
    fn test_one_matrix() {
        let mut block = VectorMergeBlock::<Matrix<1, 9, f64>, Matrix<3, 3, f64>>::default();
        let input = Matrix {
            data: [[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, &input);
        assert_eq!(
            result,
            &Matrix {
                data: [[1.], [2.], [3.], [4.], [5.], [6.], [7.], [8.], [9.]],
            }
        );
    }

    #[test]
    fn test_one_scalars() {
        let mut block = VectorMergeBlock::<Matrix<1, 1, f64>, f64>::default();
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, 1.);
        assert_eq!(result, &Matrix { data: [[1.]] });
    }

    #[test]
    fn test_two_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 18, f64>,
            (Matrix<3, 3, f64>, Matrix<3, 3, f64>),
        >::default();
        let input_a = Matrix {
            data: [[1., 2., 3.], [4., 5., 6.], [7., 8., 9.]],
        };
        let input_b = Matrix {
            data: [[10., 11., 12.], [13., 14., 15.], [16., 17., 18.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, (&input_a, &input_b));
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [5.],
                    [6.],
                    [7.],
                    [8.],
                    [9.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [14.],
                    [15.],
                    [16.],
                    [17.],
                    [18.]
                ],
            }
        );
    }

    #[test]
    fn test_two_vector() {
        let mut block =
            VectorMergeBlock::<Matrix<1, 6, f64>, (Matrix<3, 1, f64>, Matrix<1, 3, f64>)>::default(
            );
        let input_a = Matrix {
            data: [[1., 2., 3.]],
        };
        let input_b = Matrix {
            data: [[4.], [5.], [6.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, (&input_a, &input_b));
        assert_eq!(
            result,
            &Matrix {
                data: [[1.], [2.], [3.], [4.], [5.], [6.]],
            }
        );
    }

    #[test]
    fn test_two_scalars() {
        let mut block = VectorMergeBlock::<Matrix<1, 2, f64>, (f64, f64)>::default();
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, (1., 2.));
        assert_eq!(result, &Matrix { data: [[1.], [2.]] });
    }

    #[test]
    fn test_scalar_matrix() {
        let mut block = VectorMergeBlock::<Matrix<1, 5, f64>, (f64, Matrix<2, 2, f64>)>::default();
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let input = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let result = block.process(&parameters, &stub_context, (0., &input));
        assert_eq!(
            result,
            &Matrix {
                data: [[0.], [1.], [2.], [3.], [4.]],
            }
        );
    }

    #[test]
    fn test_matrix_scalar() {
        let mut block = VectorMergeBlock::<Matrix<1, 5, f64>, (Matrix<2, 2, f64>, f64)>::default();
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let input = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let result = block.process(&parameters, &stub_context, (&input, 0.));
        assert_eq!(
            result,
            &Matrix {
                data: [[1.], [2.], [3.], [4.], [0.]],
            }
        );
    }

    #[test]
    fn test_three_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 17, f64>,
            (Matrix<2, 2, f64>, Matrix<3, 3, f64>, Matrix<2, 2, f64>),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[10., 11., 12.], [13., 14., 15.], [16., 17., 18.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(&parameters, &stub_context, (&input_a, &input_b, &input_a));
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [14.],
                    [15.],
                    [16.],
                    [17.],
                    [18.],
                    [1.],
                    [2.],
                    [3.],
                    [4.]
                ],
            }
        );
    }

    #[test]
    fn test_four_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 16, f64>,
            (
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (&input_a, &input_b, &input_a, &input_b),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.]
                ],
            }
        );
    }

    #[test]
    fn test_five_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 20, f64>,
            (
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let input_c = Matrix {
            data: [[10., 11.], [12., 13.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (&input_a, &input_b, &input_a, &input_b, &input_c),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [10.],
                    [11.],
                    [12.],
                    [13.]
                ],
            }
        );
    }

    #[test]
    fn test_six_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 24, f64>,
            (
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let input_c = Matrix {
            data: [[10., 11.], [12., 13.]],
        };
        let input_d = Matrix {
            data: [[14., 15.], [16., 17.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (&input_a, &input_b, &input_a, &input_b, &input_c, &input_d),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [14.],
                    [15.],
                    [16.],
                    [17.]
                ],
            }
        );
    }

    #[test]
    fn test_seven_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 33, f64>,
            (
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<3, 3, f64>,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let input_c = Matrix {
            data: [[10., 11.], [12., 13.]],
        };
        let input_d = Matrix {
            data: [[14., 15.], [16., 17.]],
        };
        let input_e = Matrix {
            data: [[20., 21., 22.], [23., 24., 25.], [26., 27., 28.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (
                &input_a, &input_b, &input_a, &input_b, &input_c, &input_d, &input_e,
            ),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [14.],
                    [15.],
                    [16.],
                    [17.],
                    [20.],
                    [21.],
                    [22.],
                    [23.],
                    [24.],
                    [25.],
                    [26.],
                    [27.],
                    [28.],
                ],
            }
        );
    }

    #[test]
    fn test_eight_matrix() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 36, f64>,
            (
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<2, 2, f64>,
                Matrix<1, 6, f64>,
                Matrix<6, 1, f64>,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let input_c = Matrix {
            data: [[10., 11.], [12., 13.]],
        };
        let input_d = Matrix {
            data: [[14., 15.], [16., 17.]],
        };
        let input_e = Matrix {
            data: [[20.], [21.], [22.], [23.], [24.], [25.]],
        };
        let input_f = Matrix {
            data: [[26., 27., 28., 29., 30., 31.]],
        };
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (
                &input_a, &input_b, &input_a, &input_b, &input_c, &input_d, &input_e, &input_f,
            ),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [14.],
                    [15.],
                    [16.],
                    [17.],
                    [20.],
                    [21.],
                    [22.],
                    [23.],
                    [24.],
                    [25.],
                    [26.],
                    [27.],
                    [28.],
                    [29.],
                    [30.],
                    [31.],
                ],
            }
        );
    }

    #[test]
    fn test_eight_matrix_scalar() {
        let mut block = VectorMergeBlock::<
            Matrix<1, 22, f64>,
            (
                Matrix<2, 2, f64>,
                f64,
                Matrix<2, 2, f64>,
                f64,
                Matrix<2, 2, f64>,
                f64,
                Matrix<1, 6, f64>,
                f64,
            ),
        >::default();
        let input_a = Matrix {
            data: [[1., 2.], [3., 4.]],
        };
        let input_b = 1.;
        let input_c = Matrix {
            data: [[4., 3.], [2., 1.]],
        };
        let input_d = 2.;
        let input_e = Matrix {
            data: [[10., 11.], [12., 13.]],
        };
        let input_f = 3.;
        let input_g = Matrix {
            data: [[20.], [21.], [22.], [23.], [24.], [25.]],
        };
        let input_h = 4.;
        let stub_context = StubContext::default();
        let parameters = Parameters {};
        let result = block.process(
            &parameters,
            &stub_context,
            (
                &input_a, input_b, &input_c, input_d, &input_e, input_f, &input_g, input_h,
            ),
        );
        assert_eq!(
            result,
            &Matrix {
                data: [
                    [1.],
                    [2.],
                    [3.],
                    [4.],
                    [1.],
                    [4.],
                    [3.],
                    [2.],
                    [1.],
                    [2.],
                    [10.],
                    [11.],
                    [12.],
                    [13.],
                    [3.],
                    [20.],
                    [21.],
                    [22.],
                    [23.],
                    [24.],
                    [25.],
                    [4.],
                ],
            }
        );
    }
}
