use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Include dir is `c_api` in the `px4-msgs-sys` crate.
    let include_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("c_api");
    //create a list of all ".h" files in the include_dir/uORB/topics
    let include_files: Vec<_> = std::fs::read_dir(include_dir.join("uORB/topics"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "h"))
        .map(|entry| entry.path())
        .collect();

    let includes_string = include_files
        .iter()
        .map(|path| format!("#include \"{}\"", path.display()))
        .collect::<Vec<_>>()
        .join("\n");

    let message_def_builder = bindgen::builder()
        // .clang_arg("--target=armv7-unknown-linux-gnueabihf")
        .use_core()
        .header_contents("imports.h", &includes_string)
        .clang_arg("-I")
        .clang_arg(include_dir.display().to_string())
        .allowlist_var("ORB_MULTI_MAX_INSTANCES")
        .allowlist_var("GPIO_CONFIG.*")
        .allowlist_var("LOGGER_STATUS.*")
        .allowlist_var("MODE_COMPLETED.*")
        .allowlist_var("AUTOTUNE_ATTITUDE_CONTROL.*")
        .allowlist_var("EKF2_TIMESTAMPS.*")
        .allowlist_var("CAMERA_TRIGGER.*")
        .allowlist_var("GIMBAL_.*")
        .allowlist_var(".*_MESSAGE_VERSION")
        .allowlist_var(".*_QUEUE_LENGTH")
        .allowlist_var("POSITION_CONTROLLER_.*")
        .allowlist_var("DISTANCE_SENSOR.*")
        .allowlist_var("ACTUATOR_.*")
        .allowlist_var("HEATER_STATUS.*")
        .allowlist_var("ESC_REPORT.*")
        .allowlist_var("ESC_STATUS.*")
        .allowlist_var("INPUT_RC_.*")
        .allowlist_var("RC_CHANNELS.*")
        .allowlist_var("SATELLITE_INFO.*")
        .allowlist_var("TRANSPONDER_REPORT.*")
        .allowlist_var("DATAMAN_RESPONSE.*")
        .allowlist_var("SENSOR_.*")
        .allowlist_var("VEHICLE_.*")
        .allowlist_var("CONTROL_.*")
        .allowlist_var("TELEMETRY_.*")
        .allowlist_var("FLIGHT_PHASE_.*")
        .allowlist_var("MAVLINK_.*")
        .allowlist_var("ACTION_REQUEST_.*")
        .allowlist_var("TUNE_CONTROL_.*")
        .allowlist_var("ARMING_CHECK_.*")
        .allowlist_var("ESTIMATOR_STATUS_.*")
        .allowlist_var("MAG_WORKER_.*")
        .allowlist_var("FUEL_TANK_.*")
        .allowlist_var("DEBUG_ARRAY_.*")
        .allowlist_var("INTERNAL_COMBUSTION_ENGINE_.*")
        .allowlist_var("ULOG_STREAM_.*")
        .allowlist_var("CONFIG_OVERRIDES_.*")
        .allowlist_var("BATTERY_STATUS_.*")
        .allowlist_var("ORBIT_STATUS_.*")
        .allowlist_var("TIMESYNC_STATUS_.*")
        .allowlist_var("OBSTACLE_DISTANCE_.*")
        .allowlist_var("LED_CONTROL_.*")
        .allowlist_var("RTL_STATUS_.*")
        .allowlist_var("NAVIGATOR_.*")
        .allowlist_var("UAVCAN_.*")
        .allowlist_var("TAKEOFF_STATUS_.*")
        .allowlist_var("POWER_BUTTON_STATE_.*")
        .allowlist_var("VTOL_VEHICLE_STATUS_.*")
        .allowlist_var("QSHELL_REQ_.*")
        .allowlist_var("POSITION_SETPOINT_.*")
        .allowlist_var("SYSTEM_POWER_.*")
        .allowlist_var("LAUNCH_DETECTION_.*")
        .allowlist_var("REGISTER_EXT_COMPONENT.*")
        .allowlist_var("GRIPPER_COMMAND_.*")
        .allowlist_var("LANDING_GEAR_.*")
        .allowlist_var("GENERATOR_STATUS_.*")
        .allowlist_var("MANUAL_CONTROL_.*")
        .allowlist_var("GEOFENCE_.*")
        .allowlist_var("CELLULAR_STATUS_.*")
        .allowlist_var("AIRSPEED_WIND.*")
        .allowlist_var("GPS_INJECT_.*")
        .allowlist_var("MESSAGE_FORMAT_REQUEST.*")
        .allowlist_var("RC_PARAMETER_MAP_.*")
        .allowlist_var("__orb_.*")
        .allowlist_type(".*_s")
        .blocklist_item("__atomic.*")
        .blocklist_item("__pthread_.*")
        .allowlist_recursively(false);

    message_def_builder
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("message_defs.rs"))
        .unwrap();

    let orb_builder = bindgen::builder()
        // .clang_arg("--target=armv7-unknown-linux-gnueabihf")
        .use_core()
        .header_contents("imports.h", &includes_string)
        .clang_arg("-I")
        .clang_arg(include_dir.display().to_string()) // blocklist pthread types/functions
        .allowlist_function("(u*)orb_.*")
        .allowlist_type(".*_state_t")
        .allowlist_type(".*_pos_t")
        .allowlist_type("orb_id(_size)*_t")
        .allowlist_type("orb_advert_t")
        .allowlist_type("orb_metadata")
        .blocklist_item("__atomic.*")
        .blocklist_item("__pthread_.*")
        .allowlist_recursively(false);

    orb_builder
        .generate()
        .unwrap()
        .write_to_file(out_dir.join("orb.rs"))
        .unwrap();
}
