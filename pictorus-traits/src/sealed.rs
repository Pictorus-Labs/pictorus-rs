// this trait is public but unreachable from third-party crates so it cannot be implemented on
// third-party types
//
// this is used as a supertrait of the `DataType` and `Inputs` traits, which effectively limits
// the set of types that can implement those traits to the `impl` blocks defined in this crate
pub trait Sealed {}
