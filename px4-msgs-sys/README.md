# Pictorus px4-msgs-sys

This crate uses bindgen to generate Rust FFI bindings for PX4 defined C structs contained in header files in the `c_api` folder. The header files were generated from 1.16 PX4 .msg files using the px_generate_uorb_topic_files.py script in the PX4 source repository.