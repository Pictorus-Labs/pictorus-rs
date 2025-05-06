# Pictorus Blocks

This crate contains all of the blocks available in the Pictorus UI. These blocks are implemented using the traits defined in the `pictorus-traits` crate.

## Block Types

There are currently two categories of blocks defined in this crate:

- [Core Blocks](./src/core_blocks/) - These are blocks that are available on all platforms.
- [Standard Blocks](./src/std_blocks/) - These blocks are only available on platforms that support the standard library.

## Implementing Custom Blocks

The blocks in this crate can be helpful as a starting point for implementing a custom block, but are likely to be much more complex than required for a typical use case. The blocks in this crate typically support a wide variety of input/output types, and as such, need to use more complex patterns like macros and recursive traits to cover all possible cases. If you are implementing a custom block for a specific case, you can likely create a much simpler implementation by restricting the input/output types you need to support. We will be adding more examples of simple custom blocks in the future.
