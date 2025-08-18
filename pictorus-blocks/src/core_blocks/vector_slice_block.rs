use crate::{traits::MatrixOps, Scalar};
use num_traits::ToPrimitive;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

/// Parameters for the VectorSliceBlock
pub struct Parameters {
    /// Starting row to slice from
    pub rows: usize,
    /// Starting column to slice from
    pub cols: usize,
}

impl Parameters {
    pub fn new(rows: f64, cols: f64) -> Self {
        let r_usize = rows
            .to_usize()
            .expect("Failed to convert rows to usize in VectorSliceBlock Parameters");
        let c_usize = cols
            .to_usize()
            .expect("Failed to convert cols to usize in VectorSliceBlock Parameters");
        Self {
            rows: r_usize,
            cols: c_usize,
        }
    }
}

/// Returns a fixed-size slice of the input matrix starting from the specified row and column.
pub struct VectorSliceBlock<I, O> {
    pub data: OldBlockData,
    buffer: O,
    _phantom: core::marker::PhantomData<I>,
}

impl<I, O> Default for VectorSliceBlock<I, O>
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

/// This function maps the input coordinates to the output coordinates and checks their validity.
/// Returns None if the coordinates are invalid, otherwise returns the output coordinates accounting for
/// the offset provided by parameters.
fn output_coordinate_map<
    const IROWS: usize,
    const ICOLS: usize,
    const OROWS: usize,
    const OCOLS: usize,
>(
    c: usize,
    r: usize,
    parameters: &Parameters,
) -> Option<(usize, usize)> {
    // Current column is less than the starting column
    if c < parameters.cols {
        return None;
    }

    // Current row is less than the starting row
    if r < parameters.rows {
        return None;
    }

    let o_rows = r - parameters.rows;
    let o_cols = c - parameters.cols;

    if o_rows < OROWS && o_cols < OCOLS {
        return Some((o_rows, o_cols));
    }

    None
}

impl<S: Scalar, const IROWS: usize, const ICOLS: usize, const OROWS: usize, const OCOLS: usize>
    ProcessBlock for VectorSliceBlock<Matrix<IROWS, ICOLS, S>, Matrix<OROWS, OCOLS, S>>
where
    S: num_traits::Zero,
    OldBlockData: FromPass<Matrix<OROWS, OCOLS, S>>,
{
    type Inputs = Matrix<IROWS, ICOLS, S>;
    type Output = Matrix<OROWS, OCOLS, S>;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        // Attempt some static assertions for sizing
        const {
            assert!(
                IROWS >= OROWS,
                "Output slice rows are larger than input slice"
            );
        }
        const {
            assert!(
                ICOLS >= OCOLS,
                "Output slice cols are larger than input slice"
            );
        }

        // Zero the buffer each time or out of bounds access will return the last copied values
        self.buffer.data.fill([S::zero(); OROWS]);

        input.for_each(|i, i_c, i_r| {
            if let Some((o_row, o_col)) =
                output_coordinate_map::<IROWS, ICOLS, OROWS, OCOLS>(i_c, i_r, parameters)
            {
                self.buffer.data[o_col][o_row] = i;
            }
        });

        self.data = OldBlockData::from_pass(&self.buffer);
        &self.buffer
    }
}

impl<S: Scalar, const IROWS: usize, const ICOLS: usize> ProcessBlock
    for VectorSliceBlock<Matrix<IROWS, ICOLS, S>, S>
where
    S: num_traits::Zero,
    OldBlockData: FromPass<S>,
{
    type Inputs = Matrix<IROWS, ICOLS, S>;
    type Output = S;
    type Parameters = Parameters;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<'_, Self::Output> {
        // Zero the buffer each time or out of bounds access will return the last copied values
        self.buffer = S::zero();

        self.buffer = input.data.get(parameters.cols)
                                .and_then(|col| col.get(parameters.rows))
                                .copied()
                                .unwrap_or(S::zero());

        self.data = OldBlockData::from_scalar(self.buffer.into());
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vector_slice_block_1x1_scalar() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, f64>::default();
        let parameters = Parameters::new(1., 1.);

        let input = Matrix {
            data: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };

        let output = block.process(&parameters, &c, &input);
        assert_eq!(output, 6.0);

        let parameters = Parameters::new(0., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output, 1.0);

        let parameters = Parameters::new(2., 2.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output, 11.0);
    }

    #[test]
    fn test_vector_slice_block_1x1_matrix() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, Matrix<1, 1, f64>>::default();
        let parameters = Parameters::new(1., 1.);

        let input = Matrix {
            data: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };

        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[6.0]]);

        let parameters = Parameters::new(0., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[1.0]]);

        let parameters = Parameters::new(2., 2.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[11.0]]);
    }

    #[test]
    fn test_vector_slice_block_2x2() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, Matrix<2, 2, f64>>::default();
        let parameters = Parameters::new(1., 1.);

        let input = Matrix {
            data: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };

        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[6.0, 7.0], [10.0, 11.0]]);

        let parameters = Parameters::new(0., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[1.0, 2.0], [5.0, 6.0]]);

        let parameters = Parameters::new(2., 2.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[11.0, 12.0], [15.0, 16.0]]);
    }

    #[test]
    fn test_vector_slice_block_1x4() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, Matrix<1, 4, f64>>::default();
        let parameters = Parameters::new(0., 0.);

        // Data is stored in [[<T>; ROW]; COL] and accessed as [COL][ROW]
        let input = Matrix {
            data: [
                //R0   R1   R2   R3
                [1.0, 2.0, 3.0, 4.0],     // Col0
                [5.0, 6.0, 7.0, 8.0],     // Col1
                [9.0, 10.0, 11.0, 12.0],  // Col2
                [13.0, 14.0, 15.0, 16.0], // Col3
            ],
        };

        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[1.], [5.], [9.], [13.]]);

        let parameters = Parameters::new(1., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[2.], [6.], [10.], [14.]]);

        let parameters = Parameters::new(2., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[3.], [7.], [11.], [15.]]);

        let parameters = Parameters::new(3., 0.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[4.], [8.], [12.], [16.]]);
    }

    #[test]
    fn test_vector_slice_block_4x1() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, Matrix<4, 1, f64>>::default();
        let parameters = Parameters::new(0., 0.);

        // Data is stored in [[<T>; ROW]; COL] and accessed as [COL][ROW]
        let input = Matrix {
            data: [
                //R0   R1   R2   R3
                [1.0, 2.0, 3.0, 4.0],     // Col0
                [5.0, 6.0, 7.0, 8.0],     // Col1
                [9.0, 10.0, 11.0, 12.0],  // Col2
                [13.0, 14.0, 15.0, 16.0], // Col3
            ],
        };

        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[1., 2., 3., 4.]]);

        let parameters = Parameters::new(0., 1.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[5., 6., 7., 8.]]);

        let parameters = Parameters::new(0., 2.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[9., 10., 11., 12.]]);

        let parameters = Parameters::new(0., 3.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[13., 14., 15., 16.]]);
    }

    #[test]
    fn test_vector_slice_block_out_of_bounds() {
        use super::*;
        use crate::testing::StubContext;

        let c = StubContext::default();
        let mut block = VectorSliceBlock::<Matrix<4, 4, f64>, Matrix<2, 2, f64>>::default();

        // Matrix Data is stored in [[<T>; ROW]; COL] and accessed as [COL][ROW]
        let input = Matrix {
            data: [
                //R0   R1   R2   R3
                [1.0, 2.0, 3.0, 4.0],     // Col0
                [5.0, 6.0, 7.0, 8.0],     // Col1
                [9.0, 10.0, 11.0, 12.0],  // Col2
                [13.0, 14.0, 15.0, 16.0], // Col3
            ],
        };

        // Test mostly out of bounds
        let parameters = Parameters::new(3., 3.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[16.0, 0.0], [0.0, 0.0]]);

        // Test completely
        let parameters = Parameters::new(4., 4.);
        let output = block.process(&parameters, &c, &input);
        assert_eq!(output.data, [[0.0, 0.0], [0.0, 0.0]]);
    }
}
