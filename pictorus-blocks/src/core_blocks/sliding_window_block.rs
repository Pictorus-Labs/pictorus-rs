use core::fmt::Debug;
use heapless::Deque;
use pictorus_block_data::{BlockData as OldBlockData, FromPass};
#[cfg(feature = "tricore")]
use pictorus_traits::FlattenSlice as _;
use pictorus_traits::{HasIc, Matrix, Pass, PassBy, ProcessBlock, Scalar};

/// Parameters for the SlidingWindowBlock consist of the initial condition of the output. This is only
/// used until N samples have been processed.
pub struct Parameters<I> {
    initial_condition: I,
}

impl<I> Parameters<I> {
    pub fn new(initial_condition: I) -> Parameters<I> {
        Parameters { initial_condition }
    }
}

/// Concatenates samples into a signle output of length N.
///
/// The concatenation is done column wise.
/// SlidingWindowBlock takes 3 compile time arguments:
/// N = the number of samples to concatenate
/// I = the input type (i.e. Scalar or Matrix<ROWS, ICOLS, Scalar>)
/// O = the output type (i.e. Matrix<ROWS, OCOLS, Scalar>)
///
/// OCOLS must equal ICOLS * N
/// ```
/// use pictorus_traits::Matrix;
/// use pictorus_blocks::SlidingWindowBlock;
/// // Example usage for a sliding window of 3 samples:
/// let swb_single = SlidingWindowBlock::<3, f64, Matrix<1, 3, f64>>::default();
/// let swb_vector = SlidingWindowBlock::<3, Matrix<1, 3, f64>, Matrix<1, 9, f64>>::default();
/// let swb_matrix = SlidingWindowBlock::<3, Matrix<2, 2, f64>, Matrix<2, 6, f64>>::default();
/// ```
pub struct SlidingWindowBlock<const N: usize, I, O> {
    pub data: OldBlockData,
    memory: Deque<I, N>,
    buffer: O,
    _phantom: core::marker::PhantomData<I>,
}

impl<const N: usize, I, O> Default for SlidingWindowBlock<N, I, O>
where
    O: Default + Pass,
    OldBlockData: FromPass<O>,
{
    fn default() -> Self {
        SlidingWindowBlock {
            data: <OldBlockData as FromPass<O>>::from_pass(O::default().as_by()),
            memory: Deque::new(),
            buffer: O::default(),
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<const N: usize, I> HasIc for SlidingWindowBlock<N, I, Matrix<1, N, I>>
where
    I: Scalar + Debug,
    OldBlockData: FromPass<Matrix<1, N, I>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        SlidingWindowBlock {
            data: <OldBlockData as FromPass<Matrix<1, N, I>>>::from_pass(
                parameters.initial_condition.as_by(),
            ),
            memory: Deque::new(),
            buffer: parameters.initial_condition,
            _phantom: core::marker::PhantomData,
        }
    }
}

// Scalar values in, Matrix out
impl<const N: usize, I> ProcessBlock for SlidingWindowBlock<N, I, Matrix<1, N, I>>
where
    I: Scalar + Debug,
    OldBlockData: FromPass<Matrix<1, N, I>>,
{
    type Inputs = I;
    type Output = Matrix<1, N, I>;
    type Parameters = Parameters<Self::Output>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        // Initialize the memory with the initial condition
        if self.memory.is_empty() {
            self.buffer = parameters.initial_condition;
        }

        self.memory
            .push_back(input)
            .expect("SlidingWindowBlock VecDeque is full");

        for (i, value) in self.memory.iter().enumerate() {
            // Until the Deque is full adjust the index or the
            // output will fill left to right instead of right to left
            let i = i + (N - self.memory.len());
            self.buffer.data.as_flattened_mut()[i] = *value;
        }

        self.data = OldBlockData::from_pass(&self.buffer);

        if self.memory.len() == N {
            self.memory.pop_front();
        }

        &self.buffer
    }
}

impl<const N: usize, const ROWS: usize, const ICOLS: usize, const OCOLS: usize, I> HasIc
    for SlidingWindowBlock<N, Matrix<ROWS, ICOLS, I>, Matrix<ROWS, OCOLS, I>>
where
    I: Scalar + Debug,
    OldBlockData: FromPass<Matrix<ROWS, OCOLS, I>>,
{
    fn new(parameters: &Self::Parameters) -> Self {
        SlidingWindowBlock {
            data: <OldBlockData as FromPass<Matrix<ROWS, OCOLS, I>>>::from_pass(
                parameters.initial_condition.as_by(),
            ),
            memory: Deque::new(),
            buffer: parameters.initial_condition,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<const N: usize, const ROWS: usize, const ICOLS: usize, const OCOLS: usize, I> ProcessBlock
    for SlidingWindowBlock<N, Matrix<ROWS, ICOLS, I>, Matrix<ROWS, OCOLS, I>>
where
    I: Scalar + Debug,
    OldBlockData: FromPass<Matrix<ROWS, OCOLS, I>>,
{
    type Inputs = Matrix<ROWS, ICOLS, I>;
    type Output = Matrix<ROWS, OCOLS, I>;
    type Parameters = Parameters<Self::Output>;

    fn process(
        &mut self,
        parameters: &Self::Parameters,
        _context: &dyn pictorus_traits::Context,
        input: PassBy<Self::Inputs>,
    ) -> PassBy<Self::Output> {
        const {
            assert!(
                ICOLS * N == OCOLS,
                "Output matrix cols must be N * input matrix cols for SlidingWindowBlock"
            );
        }

        // Initialize the memory with the initial condition
        if self.memory.is_empty() {
            self.buffer = parameters.initial_condition;
        }

        self.memory
            .push_back(*input)
            .expect("SlidingWindowBlock VecDeque is full");

        // This appends matrices in row order, columns are consistent, but row length must be accounted for
        for (matrix_index, value) in self.memory.iter().enumerate() {
            // Until the Deque is full adjust the index or the
            // output will fill left to right instead of right to left
            let matrix_index = matrix_index + (N - self.memory.len());
            for c in 0..ICOLS {
                for r in 0..ROWS {
                    self.buffer.data[matrix_index * ICOLS + c][r] = value.data[c][r];
                }
            }
        }

        self.data = OldBlockData::from_pass(&self.buffer);

        if self.memory.len() == N {
            self.memory.pop_front();
        }

        &self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::StubContext;
    use pictorus_block_data::BlockData as OldBlockData;
    use pictorus_traits::{Matrix, ProcessBlock};

    #[test]
    fn test_sliding_window_block() {
        let c = StubContext::default();
        let mut block = SlidingWindowBlock::<3, f64, Matrix<1, 3, f64>>::default();

        let initial_condition = Matrix {
            data: [[-1.0], [-1.0], [-1.0]],
        };

        let output = block.process(&Parameters::new(initial_condition), &c, 1.0);
        assert_eq!(output.data.as_flattened(), [-1.0, -1.0, 1.0]);
        assert_eq!(block.data, OldBlockData::from_matrix(&[&[-1.0, -1.0, 1.0]]));

        let ic2 = Matrix {
            data: [[0.0], [0.0], [0.0]],
        };

        // Test that parameters with an IC are irrelevant after the first run
        let output = block.process(&Parameters::new(ic2), &c, 2.0);
        assert_eq!(output.data.as_flattened(), [-1.0, 1.0, 2.0]);
        assert_eq!(block.data, OldBlockData::from_matrix(&[&[-1.0, 1.0, 2.0]]));

        let output = block.process(&Parameters::new(ic2), &c, 3.0);
        assert_eq!(output.data.as_flattened(), [1.0, 2.0, 3.0]);
        assert_eq!(block.data, OldBlockData::from_matrix(&[&[1.0, 2.0, 3.0]]));

        let output = block.process(&Parameters::new(ic2), &c, 4.0);
        assert_eq!(output.data.as_flattened(), [2.0, 3.0, 4.0]);
        assert_eq!(block.data, OldBlockData::from_matrix(&[&[2.0, 3.0, 4.0]]));
    }

    #[test]
    fn test_sliding_window_block_vectors() {
        let c = StubContext::default();
        let mut block = SlidingWindowBlock::<3, Matrix<1, 3, f64>, Matrix<1, 9, f64>>::default();

        let ic = Matrix {
            data: [
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
                [-1.0],
            ],
        };

        let p = Parameters::new(ic);

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[1.0], [2.0], [3.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [-1.0],
                    [-1.0],
                    [-1.0],
                    [-1.0],
                    [-1.0],
                    [-1.0],
                    [1.0],
                    [2.0],
                    [3.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[&[-1.0, -1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 2.0, 3.0]])
        );

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[4.0], [5.0], [6.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [-1.0],
                    [-1.0],
                    [-1.0],
                    [1.0],
                    [2.0],
                    [3.0],
                    [4.0],
                    [5.0],
                    [6.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[&[-1.0, -1.0, -1.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]])
        );

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[7.0], [8.0], [9.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [1.0],
                    [2.0],
                    [3.0],
                    [4.0],
                    [5.0],
                    [6.0],
                    [7.0],
                    [8.0],
                    [9.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]])
        );

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[10.0], [11.0], [12.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [4.0],
                    [5.0],
                    [6.0],
                    [7.0],
                    [8.0],
                    [9.0],
                    [10.0],
                    [11.0],
                    [12.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[&[4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0]])
        );
    }

    #[test]
    fn test_sliding_window_block_matrix() {
        let c = StubContext::default();
        let mut block = SlidingWindowBlock::<3, Matrix<2, 2, f64>, Matrix<2, 6, f64>>::default();

        let ic = Matrix {
            data: [
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
                [0.0, 0.0],
            ],
        };

        let p = Parameters::new(ic);

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[1.0, 2.0], [3.0, 4.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [0.0, 0.0],
                    [0.0, 0.0],
                    [0.0, 0.0],
                    [0.0, 0.0],
                    [1.0, 2.0],
                    [3.0, 4.0],
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[
                &[0.0, 0.0, 0.0, 0.0, 1.0, 3.0],
                &[0.0, 0.0, 0.0, 0.0, 2.0, 4.0]
            ])
        );

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[5.0, 6.0], [7.0, 8.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [0.0, 0.0],
                    [0.0, 0.0],
                    [1.0, 2.0],
                    [3.0, 4.0],
                    [5.0, 6.0],
                    [7.0, 8.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[
                &[0.0, 0.0, 1.0, 3.0, 5.0, 7.0],
                &[0.0, 0.0, 2.0, 4.0, 6.0, 8.0]
            ])
        );

        let output = block.process(
            &p,
            &c,
            &Matrix {
                data: [[9.0, 10.0], [11.0, 12.0]],
            },
        );
        assert_eq!(
            output,
            &Matrix {
                data: [
                    [1.0, 2.0],
                    [3.0, 4.0],
                    [5.0, 6.0],
                    [7.0, 8.0],
                    [9.0, 10.0],
                    [11.0, 12.0]
                ]
            }
        );
        assert_eq!(
            block.data,
            OldBlockData::from_matrix(&[
                &[1.0, 3.0, 5.0, 7.0, 9.0, 11.0],
                &[2.0, 4.0, 6.0, 8.0, 10.0, 12.0]
            ])
        );
    }
}
