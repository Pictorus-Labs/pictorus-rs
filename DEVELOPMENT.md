## Publishing a new version

We generally follow the [Rust publishing guidelines](https://doc.rust-lang.org/cargo/reference/publishing.html). Eventually it would be nice to automate some of this process to reduce the number of manual steps required (could try something like [cargo-release](https://crates.io/crates/cargo-release)). A general outline of the process is as follows:

1. Update the version(s) in the `Cargo.toml` file in each crate that will be published.
1. Update the CHANGELOG.md file for each crate with the changes made since the last version.
1. Create a pull request with the changes, and merge once approved.
1. Create and push git tags for each crate that will be published. For instance, if you are publishing version `0.1.0` of the `pictorus-sim` crate, you would run:

   ```bash
   git tag -a pictorus-sim-v0.1.0 -m "Release pictorus-sim 0.1.0"
   git push origin pictorus-sim-v0.1.0
   ```

   - You would repeat this for each crate that has been updated.

1. Publish the crates to crates.io using the `cargo publish` command. For example, to publish the `pictorus-sim` crate, you would run:

   ```bash
   cargo publish -p pictorus-sim
   ```

   - This also needs to be repeated for each crate that we want to publish.
