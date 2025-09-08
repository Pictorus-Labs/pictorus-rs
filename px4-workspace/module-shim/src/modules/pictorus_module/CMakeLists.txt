px4_add_module(
	MODULE modules__pictorus_module
	MAIN pictorus_module
	SRCS
		pictorus_module.cpp
	MODULE_CONFIG
		module.yaml
	EXTERNAL
)

# Link against the static Rust FFI library  
target_link_libraries(modules__pictorus_module
	PRIVATE
	#${CMAKE_CURRENT_LIST_DIR}/../../../../target/thumbv7em-none-eabihf/release/examples/libquad_controller.a
	${CMAKE_CURRENT_LIST_DIR}/../../../../target/debug/examples/libquad_controller.a
    # ${CMAKE_CURRENT_LIST_DIR}/../../../../target/x86_64-unknown-linux-gnu/release/examples/libffi_example.a
)

