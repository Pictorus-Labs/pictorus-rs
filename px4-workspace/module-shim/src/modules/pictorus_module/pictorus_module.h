#pragma once

#include <px4_platform_common/module.h>
#include <px4_platform_common/module_params.h>
#include <uORB/SubscriptionInterval.hpp>
#include <uORB/topics/parameter_update.h>

using namespace time_literals;

extern "C" __EXPORT int pictorus_module_main(int argc, char *argv[]);


class PictorusModule : public ModuleBase<PictorusModule>, public ModuleParams
{
public:
	PictorusModule(int example_param, bool example_flag);

	virtual ~PictorusModule() = default;

	/** @see ModuleBase */
	static int task_spawn(int argc, char *argv[]);

	/** @see ModuleBase */
	static PictorusModule *instantiate(int argc, char *argv[]);

	/** @see ModuleBase */
	static int custom_command(int argc, char *argv[]);

	/** @see ModuleBase */
	static int print_usage(const char *reason = nullptr);

	/** @see ModuleBase::run() */
	void run() override;

	/** @see ModuleBase::print_status() */
	int print_status() override;

private:

	/**
	 * Check for parameter changes and update them if needed.
	 * @param parameter_update_sub uorb subscription to parameter_update
	 * @param force for a parameter update
	 */
	void parameters_update(bool force = false);


	DEFINE_PARAMETERS(
		(ParamFloat<px4::params::PICT_P_ROLL>) _pict_p_roll,
		(ParamFloat<px4::params::PICT_I_ROLL>) _pict_i_roll,
		(ParamFloat<px4::params::PICT_D_ROLL>) _pict_d_roll
	)

	// Subscriptions
	uORB::SubscriptionInterval _parameter_update_sub{ORB_ID(parameter_update), 1_s};

};
