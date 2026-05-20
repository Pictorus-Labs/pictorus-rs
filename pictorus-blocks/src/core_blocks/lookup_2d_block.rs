use core::marker::PhantomData;

use pictorus_traits::{Matrix, Pass, PassBy, ProcessBlock};

use crate::traits::Float;

/// Performs a 2D lookup against two sets of break points and a 2D table of data points.
///
/// The lookup can either be performed using bilinear interpolation or nearest neighbor
/// interpolation, depending on the `interp_method` parameter. For matrix inputs, the
/// lookup is performed element-wise, treating each pair of elements as (x,y) coordinates.
pub struct Lookup2DBlock<const NX: usize, const NY: usize, S, T>
where
    S: Float,
    T: Apply<NX, NY, S>,
{
    buffer: T,
    _unused: PhantomData<S>,
}

impl<const NX: usize, const NY: usize, S: Float, T: Apply<NX, NY, S>> ProcessBlock
    for Lookup2DBlock<NX, NY, S, T>
{
    type Inputs = (T, T); // X and Y inputs
    type Output = T; // Output has same type/dimensions as inputs
    type Parameters = Parameters<NX, NY, S>;

    fn process<'b>(
        &'b mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        inputs: pictorus_traits::PassBy<'_, Self::Inputs>,
    ) -> pictorus_traits::PassBy<'b, Self::Output> {
        let output = T::apply(&mut self.buffer, inputs, parameters);
        output
    }

    fn buffer(&self) -> PassBy<'_, Self::Output> {
        self.buffer.as_by()
    }
}

impl<const NX: usize, const NY: usize, S: Float, T: Apply<NX, NY, S>> Default
    for Lookup2DBlock<NX, NY, S, T>
{
    fn default() -> Self {
        Self {
            buffer: T::default(),
            _unused: PhantomData,
        }
    }
}

#[derive(strum::EnumString)]
pub enum InterpMethod {
    /// Bilinear interpolation
    Linear,
    /// Nearest neighbor interpolation
    Nearest,
}

/// Parameters for the Lookup2DBlock
pub struct Parameters<const NX: usize, const NY: usize, S: Float> {
    /// Interpolation method to use
    interp_method: InterpMethod,
    /// Break points for the X-axis lookup
    break_points_u1: [S; NX],
    /// Break points for the Y-axis lookup
    break_points_u2: [S; NY],
    /// 2D Data points for the lookup, stored as Matrix<NX,NY> (i.e. NX rows and NY columns)
    data_points: Matrix<NX, NY, S>,
}

impl<const NX: usize, const NY: usize, S: Float> Parameters<NX, NY, S> {
    pub fn new(
        interp_method: &str,
        break_points_u1: [S; NX],
        break_points_u2: [S; NY],
        data_points: Matrix<NX, NY, S>,
    ) -> Self {
        Self {
            interp_method: interp_method
                .parse()
                .expect("Invalid interp method. Must be Linear or Nearest"),
            break_points_u1,
            break_points_u2,
            data_points,
        }
    }
}

pub trait Apply<const NX: usize, const NY: usize, S: Float>: Pass + Default {
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<(Self, Self)>, // (X input, Y input)
        params: &Parameters<NX, NY, S>,
    ) -> PassBy<'s, Self>;
}

impl<const NX: usize, const NY: usize, S: Float> Apply<NX, NY, S> for S {
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<(Self, Self)>,
        params: &Parameters<NX, NY, S>,
    ) -> PassBy<'s, Self> {
        let (x_val, y_val) = input;
        let interp_method = &params.interp_method;

        // Clamp x input to valid range
        let x = if x_val < params.break_points_u1[0] {
            params.break_points_u1[0]
        } else if x_val >= params.break_points_u1[NX - 1] {
            params.break_points_u1[NX - 1]
        } else {
            x_val
        };

        // Clamp y input to valid range
        let y = if y_val < params.break_points_u2[0] {
            params.break_points_u2[0]
        } else if y_val >= params.break_points_u2[NY - 1] {
            params.break_points_u2[NY - 1]
        } else {
            y_val
        };

        let result = match interp_method {
            InterpMethod::Linear => bilinear_interpolation(x, y, params),
            InterpMethod::Nearest => nearest_interpolation(x, y, params),
        };
        *store = result;
        result
    }
}

impl<const NX: usize, const NY: usize, const NROWS: usize, const NCOLS: usize, S: Float>
    Apply<NX, NY, S> for Matrix<NROWS, NCOLS, S>
{
    fn apply<'s>(
        store: &'s mut Self,
        input: PassBy<(Self, Self)>,
        params: &Parameters<NX, NY, S>,
    ) -> PassBy<'s, Self> {
        let (x_input, y_input) = input;

        // For matrices, we process elementwise, creating pairs of (x, y) inputs
        // We need a mutable scalar for the Apply<S> implementation to use
        let mut dummy_store = S::default();

        // Iterate through the matrices elementwise
        for c in 0..NCOLS {
            for r in 0..NROWS {
                let x_val = x_input.data[c][r];
                let y_val = y_input.data[c][r];
                store.data[c][r] = S::apply(&mut dummy_store, (x_val, y_val), params);
            }
        }

        store.as_by()
    }
}

fn find_index<const N: usize, S: Float>(value: S, break_points_u1: &[S; N]) -> usize {
    // Find the index where value falls between break_points_u1[index-1] and break_points_u1[index]
    // Assumes break_points_u1 is monotonically increasing
    break_points_u1
        .iter()
        .enumerate()
        .skip(1) // Skip the first element since we want to start at index 1
        .find(|&(_, &point)| value < point)
        .map(|(i, _)| i)
        .unwrap_or(N - 1) // If no match found, return the last valid index (N - 1)
}

fn bilinear_interpolation<const NX: usize, const NY: usize, S: Float>(
    x: S,
    y: S,
    params: &Parameters<NX, NY, S>,
) -> S {
    // Find indices for x and y
    let x_idx = find_index(x, &params.break_points_u1);
    let y_idx = find_index(y, &params.break_points_u2);

    // Handle edge cases where we're at/beyond the limits
    if x >= params.break_points_u1[NX - 1] && y >= params.break_points_u2[NY - 1] {
        return params.data_points.data[NY - 1][NX - 1];
    }
    if x >= params.break_points_u1[NX - 1] {
        // Interpolate only in Y dimension
        return linear_interpolate_1d(
            y,
            params.break_points_u2[y_idx - 1],
            params.break_points_u2[y_idx],
            params.data_points.data[y_idx - 1][NX - 1],
            params.data_points.data[y_idx][NX - 1],
        );
    }
    if y >= params.break_points_u2[NY - 1] {
        // Interpolate only in X dimension
        return linear_interpolate_1d(
            x,
            params.break_points_u1[x_idx - 1],
            params.break_points_u1[x_idx],
            params.data_points.data[NY - 1][x_idx - 1],
            params.data_points.data[NY - 1][x_idx],
        );
    }

    // Get the four corner points
    let x1 = params.break_points_u1[x_idx - 1];
    let x2 = params.break_points_u1[x_idx];
    let y1 = params.break_points_u2[y_idx - 1];
    let y2 = params.break_points_u2[y_idx];

    let q11 = params.data_points.data[y_idx - 1][x_idx - 1];
    let q12 = params.data_points.data[y_idx][x_idx - 1];
    let q21 = params.data_points.data[y_idx - 1][x_idx];
    let q22 = params.data_points.data[y_idx][x_idx];

    // Perform bilinear interpolation
    // First interpolate in x-direction
    let r1 = linear_interpolate_1d(x, x1, x2, q11, q21);
    let r2 = linear_interpolate_1d(x, x1, x2, q12, q22);

    // Then interpolate in y-direction
    linear_interpolate_1d(y, y1, y2, r1, r2)
}

fn linear_interpolate_1d<S: Float>(x: S, x1: S, x2: S, y1: S, y2: S) -> S {
    let t = (x - x1) / (x2 - x1);
    y1 + t * (y2 - y1)
}

fn nearest_interpolation<const NX: usize, const NY: usize, S: Float>(
    x: S,
    y: S,
    params: &Parameters<NX, NY, S>,
) -> S {
    // Find indices for x and y
    let x_idx = find_index(x, &params.break_points_u1);
    let y_idx = find_index(y, &params.break_points_u2);

    // Calculate distances to neighboring points for x
    let x_dist_low = x - params.break_points_u1[x_idx - 1];
    let x_dist_high = params.break_points_u1[x_idx] - x;

    // Determine nearest x index
    let nearest_x_idx = if x_dist_low <= x_dist_high {
        x_idx - 1
    } else {
        x_idx
    };

    // Calculate distances to neighboring points for y
    let y_dist_low = y - params.break_points_u2[y_idx - 1];
    let y_dist_high = params.break_points_u2[y_idx] - y;

    // Determine nearest y index
    let nearest_y_idx = if y_dist_low <= y_dist_high {
        y_idx - 1
    } else {
        y_idx
    };

    // Return the value at the nearest coordinate
    params.data_points.data[nearest_y_idx][nearest_x_idx]
}

#[cfg(test)]
mod tests {
    use crate::testing::StubContext;

    use super::*;

    #[test]
    fn test_lookup_2d_default_buffer_no_panic() {
        let block = Lookup2DBlock::<3, 3, f64, f64>::default();
        assert_eq!(block.buffer(), 0.0);
    }

    #[test]
    fn test_scalar_linear() {
        let ctxt = StubContext::default();

        // Create a 3x3 lookup table
        // X breakpoints: [0.0, 1.0, 2.0]
        // Y breakpoints: [0.0, 10.0, 20.0]
        // Data:
        // [  0.0,  10.0,  20.0 ]
        // [ 10.0,  20.0,  30.0 ]
        // [ 20.0,  30.0,  40.0 ]

        let break_points_u1 = [0.0, 1.0, 2.0];
        let break_points_u2 = [0.0, 10.0, 20.0];

        // Create data points as a 3x3 matrix (column-major storage).
        // This table is symmetric, so column-major == row-major:
        //   row x=0: [0, 10, 20]
        //   row x=1: [10, 20, 30]
        //   row x=2: [20, 30, 40]
        let data_points = Matrix {
            data: [[0.0, 10.0, 20.0], [10.0, 20.0, 30.0], [20.0, 30.0, 40.0]],
        };

        let params = Parameters::new("Linear", break_points_u1, break_points_u2, data_points);

        let mut block = Lookup2DBlock::<3, 3, f64, f64>::default();

        // Test exact corner points
        let res = block.process(&params, &ctxt, (0.0, 0.0));
        assert_eq!(res, 0.0);
        assert_eq!(block.buffer(), res);

        let res = block.process(&params, &ctxt, (0.0, 20.0));
        assert_eq!(res, 20.0);

        let res = block.process(&params, &ctxt, (2.0, 0.0));
        assert_eq!(res, 20.0);

        let res = block.process(&params, &ctxt, (2.0, 20.0));
        assert_eq!(res, 40.0);

        // Test midpoints along edges
        let res = block.process(&params, &ctxt, (1.0, 0.0));
        assert_eq!(res, 10.0);

        let res = block.process(&params, &ctxt, (0.0, 10.0));
        assert_eq!(res, 10.0);

        // Test center point
        let res = block.process(&params, &ctxt, (1.0, 10.0));
        assert_eq!(res, 20.0);

        // Test arbitrary point for bilinear interpolation
        let res = block.process(&params, &ctxt, (0.5, 5.0));
        // For point (0.5, 5.0) with surrounding values:
        // (0,0)=0.0, (0,10)=10.0, (1,0)=10.0, (1,10)=20.0
        // First interpolate along X:
        //   at y=0: 0.0 + 0.5*(10.0-0.0) = 5.0
        //   at y=10: 10.0 + 0.5*(20.0-10.0) = 15.0
        // Then interpolate along Y:
        //   5.0 + (5.0/10.0)*(15.0-5.0) = 5.0 + 0.5*10.0 = 10.0
        assert_eq!(res, 10.0);

        // Test clamping at boundaries
        let res = block.process(&params, &ctxt, (-1.0, -5.0));
        assert_eq!(res, 0.0);

        let res = block.process(&params, &ctxt, (3.0, 25.0));
        assert_eq!(res, 40.0);
    }

    #[test]
    fn test_scalar_nearest() {
        let ctxt = StubContext::default();

        // Create the same lookup table as above but with nearest neighbor interpolation
        let break_points_u1 = [0.0, 1.0, 2.0];
        let break_points_u2 = [0.0, 10.0, 20.0];

        let data_points = Matrix {
            data: [[0.0, 10.0, 20.0], [10.0, 20.0, 30.0], [20.0, 30.0, 40.0]],
        };

        let params = Parameters::new("Nearest", break_points_u1, break_points_u2, data_points);

        let mut block = Lookup2DBlock::<3, 3, f64, f64>::default();

        // Test exact corner points
        let res = block.process(&params, &ctxt, (0.0, 0.0));
        assert_eq!(res, 0.0);

        // Test points closer to specific grid points
        let res = block.process(&params, &ctxt, (0.4, 4.9));
        assert_eq!(res, 0.0); // Closest to (0,0)

        let res = block.process(&params, &ctxt, (0.6, 4.9));
        assert_eq!(res, 10.0); // Closest to (1,0)

        let res = block.process(&params, &ctxt, (0.4, 5.1));
        assert_eq!(res, 10.0); // Closest to (0,10)

        let res = block.process(&params, &ctxt, (0.6, 5.1));
        assert_eq!(res, 20.0); // Closest to (1,10)

        // Test clamping at boundaries
        let res = block.process(&params, &ctxt, (-1.0, -5.0));
        assert_eq!(res, 0.0);

        let res = block.process(&params, &ctxt, (3.0, 25.0));
        assert_eq!(res, 40.0);
    }

    #[test]
    fn test_matrix_linear() {
        let ctxt = StubContext::default();

        // Create the same lookup table as previous tests
        let break_points_u1 = [0.0, 1.0, 2.0];
        let break_points_u2 = [0.0, 10.0, 20.0];

        let data_points = Matrix {
            data: [[0.0, 10.0, 20.0], [10.0, 20.0, 30.0], [20.0, 30.0, 40.0]],
        };

        let params = Parameters::new("Linear", break_points_u1, break_points_u2, data_points);

        let mut block = Lookup2DBlock::<3, 3, f64, Matrix<2, 2, f64>>::default();

        // Create input matrices for X and Y coordinates
        let x_input = Matrix {
            data: [[0.0, 1.0], [0.5, 2.0]],
        };

        let y_input = Matrix {
            data: [[0.0, 10.0], [5.0, 20.0]],
        };

        let res = block.process(&params, &ctxt, (&x_input, &y_input));

        // Expected results based on the lookup table:
        // (0.0, 0.0) -> 0.0
        // (1.0, 10.0) -> 20.0
        // (0.5, 5.0) -> 10.0
        // (2.0, 20.0) -> 40.0
        let expected = Matrix {
            data: [[0.0, 20.0], [10.0, 40.0]],
        };

        assert_eq!(res.data, expected.data);
        assert_eq!(block.buffer().data, expected.data);
    }

    #[test]
    fn test_matrix_nearest() {
        let ctxt = StubContext::default();

        // Create the same lookup table but with nearest neighbor interpolation
        let break_points_u1 = [0.0, 1.0, 2.0];
        let break_points_u2 = [0.0, 10.0, 20.0];

        let data_points = Matrix {
            data: [[0.0, 10.0, 20.0], [10.0, 20.0, 30.0], [20.0, 30.0, 40.0]],
        };

        let params = Parameters::new("Nearest", break_points_u1, break_points_u2, data_points);

        let mut block = Lookup2DBlock::<3, 3, f64, Matrix<2, 2, f64>>::default();

        // Create input matrices for X and Y coordinates
        let x_input = Matrix {
            data: [[0.4, 0.6], [1.4, 1.6]],
        };

        let y_input = Matrix {
            data: [[4.9, 5.1], [14.9, 15.1]],
        };

        let res = block.process(&params, &ctxt, (&x_input, &y_input));

        // Expected results based on nearest neighbors:
        // (0.4, 4.9) -> closest to (0,0) -> 0.0
        // (0.6, 5.1) -> closest to (1,10) -> 20.0
        // (1.4, 14.9) -> closest to (1,10) -> 20.0
        // (1.6, 15.1) -> closest to (2,20) -> 40.0
        let expected = Matrix {
            data: [[0.0, 20.0], [20.0, 40.0]],
        };

        assert_eq!(res.data, expected.data);
        assert_eq!(block.buffer().data, expected.data);
    }
}
