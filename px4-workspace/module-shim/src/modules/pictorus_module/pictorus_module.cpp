#include "pictorus_module.h"
#include "rust_interface.h"
#include "pictorus.h"
#include <px4_platform_common/getopt.h>
#include <px4_platform_common/log.h>
#include <px4_platform_common/posix.h>
#include <cassert>
#include <uORB/topics/actuator_motors.h>

// Helper function to convert FfiError to human-readable string
static const char* ffi_error_to_string(FfiError error) {
    switch (error) {
        case FFI_SUCCESS:
            return "Success";
        case FFI_MESSAGE_LENGTH_MISMATCH:
            return "Message length mismatch";
        case FFI_UNADVERTISED_MESSAGE:
            return "Message type not advertised";
        case FFI_UNSUBSCRIBED_MESSAGE:
            return "Message type not subscribed";
        case FFI_INVALID_MESSAGE_INDEX:
            return "Invalid message index";
        case FFI_NULL_ARGUMENT:
            return "Null argument passed to function";
        default:
            return "Unknown error";
    }
}

constexpr size_t MAX_MESSAGES = 16; // Maximum number of messages we can publish/subscribe to
constexpr size_t MAX_MESSAGE_SIZE = 1024; // Maximum size for any uORB message (bytes)
constexpr unsigned int LOOP_INTERVAL_US = 10000; // 10ms loop interval

class OrbSubscription {
public:
    OrbSubscription() = default;
    explicit OrbSubscription(orb_id_t id) : id_(id), handle_(orb_subscribe(id)) {
        if (handle_ < 0) {
            PX4_ERR("Failed to subscribe to message");
            handle_ = -1;
        }
    }

    // Delete copy operations
    OrbSubscription(const OrbSubscription&) = delete;
    OrbSubscription& operator=(const OrbSubscription&) = delete;

    // Move constructor
    OrbSubscription(OrbSubscription&& other) noexcept
        : id_(other.id_), handle_(other.handle_) {
        other.handle_ = -1; // Invalidate the moved-from object
    }

    // Move assignment operator
    OrbSubscription& operator=(OrbSubscription&& other) noexcept {
        if (this != &other) {
            // Clean up current resource
            if (handle_ >= 0) {
                orb_unsubscribe(handle_);
            }

            // Move from other
            id_ = other.id_;
            handle_ = other.handle_;
            other.handle_ = -1; // Invalidate the moved-from object
        }
        return *this;
    }

    ~OrbSubscription() {
        if (handle_ >= 0) {
            orb_unsubscribe(handle_);
        }
    }

    bool is_valid() const { return handle_ >= 0; }
    orb_id_t id() const { return id_; }
    
    bool check_updated() const {
        if (!is_valid()) return false;
        bool updated = false;
        return orb_check(handle_, &updated) >= 0 && updated;
    }
    
    bool copy_data(void* buffer) const {
        if (!is_valid() || !buffer) return false;
        return orb_copy(id_, handle_, buffer) >= 0;
    }

private:
    orb_id_t id_ = nullptr;
    int handle_ = -1;
};

// RAII wrapper for uORB publications
class OrbPublication {
public:
    OrbPublication() = default;
    explicit OrbPublication(orb_id_t id, const void* initial_data) 
        : id_(id), handle_(orb_advertise(id, initial_data)) {
        if (!handle_) {
            PX4_ERR("Failed to advertise message");
        }
    }

    ~OrbPublication() = default; // uORB handles cleanup
    bool is_valid() const { return handle_ != nullptr; }
    orb_id_t id() const { return id_; }
    
    bool publish(const void* data) const {
        if (!is_valid() || !data) return false;
        return orb_publish(id_, handle_, data) >= 0;
    }

private:
    orb_id_t id_ = nullptr;
    orb_advert_t handle_ = nullptr;
};

// Message management helper classes
class InputManager {
public:
    void process_input_messages() {
        static uint8_t buffer[MAX_MESSAGE_SIZE];
        size_t input_count = 0;
        FfiError error = rust_get_input_message_count(&input_count);
        if (error != FFI_SUCCESS) {
            PX4_ERR("Failed to get input message count: %s", ffi_error_to_string(error));
            return;
        }
        assert(input_count <= MAX_MESSAGES);
        
        for (size_t i = 0; i < input_count; ++i) {
            orb_id_t message_id;
            error = rust_get_input_message_id(i, &message_id);
            if (error != FFI_SUCCESS) {
                PX4_ERR("Failed to get input message ID at index %zu: %s", i, ffi_error_to_string(error));
                continue;
            }
            
            auto& sub = ensure_subscription(message_id);
            if (sub.check_updated()) {
                // Get message size from orb_metadata
                size_t message_size = message_id->o_size;
                
                // Validate message size fits in our buffer,
                // We should actually validate this at subscription time
                if (message_size > MAX_MESSAGE_SIZE) {
                    PX4_ERR("Message size %zu exceeds MAX_MESSAGE_SIZE %zu for topic '%s'", 
                           message_size, MAX_MESSAGE_SIZE, message_id->o_name);
                    continue;
                }
                
                if (sub.copy_data(buffer)) {
                    // Send data to Rust
                    FfiError result = rust_write_input_message(message_id, buffer, message_size);
                    if (result != FFI_SUCCESS) {
                        PX4_ERR("Failed to write input message for topic '%s': %s", 
                               message_id->o_name, ffi_error_to_string(result));
                    }
                }
            }
        }
        // PX4_INFO("Finished processing input messages");
    }

private:
    OrbSubscription subscriptions_[MAX_MESSAGES];
    
    OrbSubscription& ensure_subscription(orb_id_t id) {
        // Find existing subscription
        for (size_t i = 0; i < MAX_MESSAGES; ++i) {
            if (subscriptions_[i].is_valid() && subscriptions_[i].id() == id) {
                return subscriptions_[i];
            }
        }
        
        // Find empty slot
        for (size_t i = 0; i < MAX_MESSAGES; ++i) {
            if (!subscriptions_[i].is_valid()) {
                PX4_INFO("Subscribing to input message with id %p", id);
                subscriptions_[i] = OrbSubscription(id);
                return subscriptions_[i];
            }
        }
        
        PX4_ERR("Too many input message subscriptions (max %zu)", MAX_MESSAGES);
        static OrbSubscription invalid_sub;
        return invalid_sub;
    }
};

class OutputManager {
public:
    void process_output_messages() {
        // PX4_INFO("Processing output messages");
        // Buffer to hold message data
        static uint8_t buffer[MAX_MESSAGE_SIZE];

        size_t output_count = 0;
        FfiError error = rust_get_output_message_count(&output_count);
        if (error != FFI_SUCCESS) {
            PX4_ERR("Failed to get output message count: %s", ffi_error_to_string(error));
            return;
        }
        assert(output_count <= MAX_MESSAGES);

        for (size_t i = 0; i < output_count; ++i) {
            orb_id_t message_id;
            error = rust_get_output_message_id(i, &message_id);
            if (error != FFI_SUCCESS) {
                PX4_ERR("Failed to get output message ID at index %zu: %s", i, ffi_error_to_string(error));
                continue;
            }
            
            // Check if Rust has updated this output message
            bool has_update = false;
            error = rust_output_message_has_update(message_id, &has_update);
            if (error != FFI_SUCCESS) {
                PX4_ERR("Failed to check update status for topic '%s': %s", 
                       message_id->o_name, ffi_error_to_string(error));
                continue;
            }
            
            if (has_update) {
                // PX4_INFO("Output message for topic '%s' has update", message_id->o_name);
                // Get message size from orb_metadata
                size_t message_size = message_id->o_size;
                
                // Validate message size fits in our buffer
                if (message_size > MAX_MESSAGE_SIZE) {
                    PX4_ERR("Message size %zu exceeds MAX_MESSAGE_SIZE %zu for topic '%s'", 
                           message_size, MAX_MESSAGE_SIZE, message_id->o_name);
                    continue;
                }

                size_t bytes_read = 0;
                // Read data from Rust
                FfiError result = rust_read_output_message(message_id, buffer, MAX_MESSAGE_SIZE, &bytes_read);
                if (result == FFI_SUCCESS && bytes_read == message_size) {
                    // Publish to uORB

                    // //For Debug print hex dump of message
                    // if (message_id == ORB_ID(actuator_motors)) {
                    //     char hex_dump[message_size * 2 + 1];
                    //     for (size_t j = 0; j < message_size; ++j) {
                    //         sprintf(&hex_dump[j * 2], "%02X", buffer[j]);
                    //     }
                    //     hex_dump[message_size * 2] = '\0';
                    //     PX4_INFO("Publishing actuator_motors message: %s", hex_dump);
                    // }
                    auto& pub = ensure_publication(message_id, buffer);
                    if (pub.is_valid()) {
                        pub.publish(buffer);
                    }
                } else if (result != FFI_SUCCESS) {
                    PX4_ERR("Failed to read output message for topic '%s': %s", 
                           message_id->o_name, ffi_error_to_string(result));
                } else {
                    PX4_ERR("Size mismatch reading output message for topic '%s': expected %zu bytes, got %zu bytes", 
                           message_id->o_name, message_size, bytes_read);
                }
            }
        }
    }

private:
    OrbPublication publications_[MAX_MESSAGES];
    
    OrbPublication& ensure_publication(orb_id_t id, const void* initial_data) {
        // Find existing publication
        for (size_t i = 0; i < MAX_MESSAGES; ++i) {
            if (publications_[i].is_valid() && publications_[i].id() == id) {
                return publications_[i];
            }
        }
        
        // Find empty slot
        for (size_t i = 0; i < MAX_MESSAGES; ++i) {
            if (!publications_[i].is_valid()) {
                PX4_INFO("Advertising output message with id %p", id);
                publications_[i] = OrbPublication(id, initial_data);
                return publications_[i];
            }
        }
        
        PX4_ERR("Too many output message publications (max %zu)", MAX_MESSAGES);
        static OrbPublication invalid_pub;
        return invalid_pub;
    }
};



int PictorusModule::print_status()
{
	PX4_INFO("Running");
	// TODO: print additional runtime information about the state of the module

	return 0;
}

int PictorusModule::custom_command(int argc, char *argv[])
{
	/*
	if (!is_running()) {
		print_usage("not running");
		return 1;
	}

	// additional custom commands can be handled like this:
	if (!strcmp(argv[0], "do-something")) {
		get_instance()->do_something();
		return 0;
	}
	 */

	return print_usage("unknown command");
}


int PictorusModule::task_spawn(int argc, char *argv[])
{
	_task_id = px4_task_spawn_cmd("module",
				      SCHED_DEFAULT,
				      SCHED_PRIORITY_DEFAULT,
				      1024,
				      (px4_main_t)&run_trampoline,
				      (char *const *)argv);

	if (_task_id < 0) {
		_task_id = -1;
		return -errno;
	}

	return 0;
}

PictorusModule *PictorusModule::instantiate(int argc, char *argv[])
{
	int example_param = 0;
	bool example_flag = false;
	bool error_flag = false;

	int myoptind = 1;
	int ch;
	const char *myoptarg = nullptr;

	// parse CLI arguments
	while ((ch = px4_getopt(argc, argv, "p:f", &myoptind, &myoptarg)) != EOF) {
		switch (ch) {
		case 'p':
			example_param = (int)strtol(myoptarg, nullptr, 10);
			break;

		case 'f':
			example_flag = true;
			break;

		case '?':
			error_flag = true;
			break;

		default:
			PX4_WARN("unrecognized flag");
			error_flag = true;
			break;
		}
	}

	if (error_flag) {
		return nullptr;
	}

	PictorusModule *instance = new PictorusModule(example_param, example_flag);

	if (instance == nullptr) {
		PX4_ERR("alloc failed");
	}

	return instance;
}

PictorusModule::PictorusModule(int example_param, bool example_flag)
	: ModuleParams(nullptr)
{
}

void PictorusModule::run()
{
   
    PX4_INFO("PictorusModule started");
    parameters_update(true);
    
    // Initialize Pictorus Application
    AppInterface* app = app_interface_new();
    if (!app) {
        PX4_ERR("Failed to create Pictorus AppInterface");
        return;
    }

    
    // Create message managers
    InputManager input_manager;
    OutputManager output_manager;


    hrt_abstime loop_start_time;
    
    while (!should_exit()) {
        loop_start_time = hrt_absolute_time();

        // Update parameters if needed
        parameters_update();
        
        // Process input messages - get data from uORB and send to Rust
        input_manager.process_input_messages();
        
        // Run Rust computation step
        // PX4_INFO("Stepping Rust application");
        app_interface_update(app, hrt_absolute_time());
        // PX4_INFO("Finished stepping Rust application");

        // Process output messages - get data from Rust and send to uORB
        output_manager.process_output_messages();

        
        // Sleep for loop interval
        px4_usleep(LOOP_INTERVAL_US - (hrt_absolute_time() - loop_start_time));
    }
    app_interface_free(app);
    
    PX4_INFO("PictorusModule stopped");

}

void PictorusModule::parameters_update(bool force)
{
	// check for parameter updates
	if (_parameter_update_sub.updated() || force) {
		// clear update
		parameter_update_s update;
		_parameter_update_sub.copy(&update);

		// update parameters from storage
		updateParams();
	}
}

int PictorusModule::print_usage(const char *reason)
{
	if (reason) {
		PX4_WARN("%s\n", reason);
	}

	PRINT_MODULE_DESCRIPTION(
		R"DESCR_STR(
### Description
Section that describes the provided module functionality.

This is a template for a module running as a task in the background with start/stop/status functionality.

### Implementation
Section describing the high-level implementation of this module.

### Examples
CLI usage example:
$ module start -f -p 42

)DESCR_STR");

	PRINT_MODULE_USAGE_NAME("module", "pictorus");
	PRINT_MODULE_USAGE_COMMAND("start");
	PRINT_MODULE_USAGE_PARAM_FLAG('f', "Optional example flag", true);
	PRINT_MODULE_USAGE_PARAM_INT('p', 0, 0, 1000, "Optional example parameter", true);
	PRINT_MODULE_USAGE_DEFAULT_COMMANDS();

	return 0;
}

int pictorus_module_main(int argc, char *argv[])
{
	return PictorusModule::main(argc, argv);
}
