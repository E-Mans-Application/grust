grust_compiler_top::prelude! {}

#[test]
fn should_compile_speed_limiter() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/speed_limiter.rs", mode = test]

        // # Imports
        import signal  car::hmi::speed_limiter::activation : ActivationRequest;
        import signal  car::hmi::speed_limiter::set_speed : float;
        import signal  car::adas::speed : float;
        import signal  car::adas::vacuum_brake : VacuumBrakeState;
        import signal  car::adas::kickdown: KickdownState;
        import signal  car::adas::vdc: VdcState;

        export signal car::adas::speed_limiter::in_regulation : bool;
        export signal car::adas::speed_limiter::v_set         : float;

        // # Types

        // Hysterisis for speed.
        struct Hysterisis {
            value: float,
            flag: bool,
        }
        function new_hysterisis(value: float) -> Hysterisis {
            return Hysterisis { value: value, flag: true };
        }

        // Enumerates the kinds of activation resquests.
        enum ActivationRequest { Off, On, Initialization, StandBy }

        // Vehicle dynamic control states.
        enum VdcState { On, Off }

        // Vacuum brake state.
        enum VacuumBrakeState { BelowMinLevel, AboveMinLevel }

        // Tells if the driver presses down hard on the accelerator or not.
        enum KickdownState{ Deactivated, Activated }

        // Speed limiter states.
        enum SpeedLimiter {
            Off,
            On,
            Fail,
        }

        // Speed limiter 'On' states.
        enum SpeedLimiterOn {
            StandBy,
            Actif,
            OverrideVoluntary,
            OverrideInvoluntary,
        }

        // # Computation functions

        // Updates the previous hysterisis according to the current speed and the calibration.
        // Determines if the current speed is within regulation.
        function update_hysterisis(prev_hyst: Hysterisis, speed: float, v_set: float) -> Hysterisis {
            let activation_threshold: float = v_set*0.99;
            let deactivation_threshold: float = v_set*0.98;
            let flag: bool = if prev_hyst.flag && speed <= deactivation_threshold
                then false
                else if !prev_hyst.flag && speed >= activation_threshold
                    then true
                    else prev_hyst.flag;
            let new_hysterisis: Hysterisis = Hysterisis { value: speed, flag: flag };
            return new_hysterisis;
        }

        // Tells if we are in regulation.
        function in_regulation(hysterisis: Hysterisis) -> bool {
            return hysterisis.flag;
        }

        // Gets a diagnostic from the state of the speed limiter.
        function into_diagnostic(to_be_defined: int) -> int {
            return to_be_defined;
        }

        // Threshold for the limit speed requested by the driver.
        function threshold_set_speed(set_speed: float) -> float {
            let ceiled_speed: float = if set_speed > 150.0 then 150.0 else set_speed;
            let grounded_speed: float = if set_speed < 10.0 then 10.0 else ceiled_speed;
            return grounded_speed;
        }

        // # Transition tests functions

        // Speed limiter 'Off' condition.
        function off_condition(activation_req: ActivationRequest, vdc_disabled: VdcState) -> bool {
            return activation_req == ActivationRequest::Off || vdc_disabled == VdcState::Off;
        }

        // Speed limiter 'On' condition.
        function on_condition(activation_req: ActivationRequest) -> bool {
            return activation_req == ActivationRequest::On || activation_req == ActivationRequest::Initialization;
        }

        // Speed limiter 'Activation' condition.
        function activation_condition(activation_req: ActivationRequest, vacuum_brake_state: VacuumBrakeState, v_set: float) -> bool {
            return activation_req == ActivationRequest::On
                && vacuum_brake_state != VacuumBrakeState::BelowMinLevel
                && v_set > 0.0;
        }

        // Speed limiter 'Exit Override' condition.
        function exit_override_condition(activation_req: ActivationRequest, kickdown: KickdownState, v_set: float, speed: float) -> bool {
            return on_condition(activation_req) && kickdown != KickdownState::Activated && speed <= v_set;
        }

        // Speed limiter 'Involuntary Override' condition.
        function involuntary_override_condition(activation_req: ActivationRequest, kickdown: KickdownState, v_set: float, speed: float) -> bool {
            return on_condition(activation_req) && kickdown != KickdownState::Activated && speed > v_set;
        }

        // Speed limiter 'Voluntary Override' condition.
        function voluntary_override_condition(activation_req: ActivationRequest, kickdown: KickdownState) -> bool {
            return on_condition(activation_req) && kickdown == KickdownState::Activated;
        }

        // Speed limiter 'StandBy' condition.
        function standby_condition(activation_req: ActivationRequest, vacuum_brake_state: VacuumBrakeState, v_set: float) -> bool {
            return activation_req == ActivationRequest::StandBy
                || vacuum_brake_state == VacuumBrakeState::BelowMinLevel
                || v_set <= 0.0;
        }

        // # Components

        // Processes the speed setted by the driver.
        component process_set_speed(set_speed: float) -> (v_set: float, v_update: bool) {
            init v_set = 0.0;
            v_set = threshold_set_speed(set_speed);
            v_update = prev_v_set != v_set;
            let prev_v_set: float = last v_set;
        }

        // Speed limiter state machine.
        component speed_limiter(
            activation_req: ActivationRequest,
            vacuum_brake_state: VacuumBrakeState,
            kickdown: KickdownState,
            vdc_disabled: VdcState,
            speed: float,
            v_set: float,
            clock: float?,
        ) -> (
            state: SpeedLimiter,
            on_state: SpeedLimiterOn,
            in_regulation: bool,
            state_update: bool
        ) {
            let failure: bool = false;
            let prev_state: SpeedLimiter = last state;
            let prev_on_state: SpeedLimiterOn = last on_state;
            when {
                init => {
                    (state, on_state, in_regulation) = (SpeedLimiter::Off, SpeedLimiterOn::StandBy, true);
                }
                clock? => {
                    match prev_state {
                        _ if off_condition(activation_req, vdc_disabled) => {
                            state = SpeedLimiter::Off;
                            on_state = prev_on_state;
                            in_regulation = false;
                        },
                        SpeedLimiter::Off if on_condition(activation_req) => {
                            match failure {
                                true => {
                                    state = SpeedLimiter::Fail;
                                    on_state = prev_on_state;
                                    in_regulation = false;
                                },
                                false => {
                                    state = SpeedLimiter::On;
                                    on_state = SpeedLimiterOn::StandBy;
                                    in_regulation = true;
                                },
                            }
                        },
                        SpeedLimiter::On if failure => {
                            state = SpeedLimiter::Fail;
                            on_state = prev_on_state;
                            in_regulation = false;
                        },
                        SpeedLimiter::Fail if !failure => {
                            state = SpeedLimiter::On;
                            on_state = SpeedLimiterOn::StandBy;
                            in_regulation = true;
                        },
                        SpeedLimiter::On => {
                            state = prev_state;
                            (on_state, in_regulation) = speed_limiter_on(
                                prev_on_state, activation_req,
                                vacuum_brake_state,
                                kickdown, speed, v_set,
                            );
                        },
                        _ => {
                            state = prev_state;
                            on_state = prev_on_state;
                            in_regulation = last in_regulation;
                        },
                    }
                }
            }
            state_update = state != prev_state || on_state != prev_on_state;
        }

        // Speed limiter 'On' state machine.
        component speed_limiter_on(
            prev_on_state: SpeedLimiterOn,
            activation_req: ActivationRequest,
            vacuum_brake_state: VacuumBrakeState,
            kickdown: KickdownState,
            speed: float,
            v_set: float,
        ) -> (
            on_state: SpeedLimiterOn,
            in_reg: bool
        ) {
            init hysterisis = new_hysterisis(0.0);
            let prev_hysterisis: Hysterisis = last hysterisis;
            in_reg = in_regulation(hysterisis);
            match prev_on_state {
                SpeedLimiterOn::StandBy if activation_condition(activation_req, vacuum_brake_state, v_set) => {
                    on_state = SpeedLimiterOn::Actif;
                    let hysterisis: Hysterisis = new_hysterisis(0.0);
                },
                SpeedLimiterOn::OverrideVoluntary if exit_override_condition(activation_req, kickdown, v_set, speed) => {
                    on_state = SpeedLimiterOn::Actif;
                    let hysterisis: Hysterisis = new_hysterisis(0.0);
                },
                SpeedLimiterOn::OverrideInvoluntary if exit_override_condition(activation_req, kickdown, v_set, speed) => {
                    on_state = SpeedLimiterOn::Actif;
                    let hysterisis: Hysterisis = new_hysterisis(0.0);
                },
                SpeedLimiterOn::OverrideVoluntary if involuntary_override_condition(activation_req, kickdown, v_set, speed) => {
                    on_state = SpeedLimiterOn::OverrideInvoluntary;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
                SpeedLimiterOn::Actif if voluntary_override_condition(activation_req, kickdown) => {
                    on_state = SpeedLimiterOn::OverrideVoluntary;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
                SpeedLimiterOn::Actif if standby_condition(activation_req, vacuum_brake_state, v_set) => {
                    on_state = SpeedLimiterOn::StandBy;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
                SpeedLimiterOn::Actif => {
                    on_state = prev_on_state;
                    let hysterisis: Hysterisis = update_hysterisis(prev_hysterisis, speed, v_set);
                },
                _ => {
                    on_state = prev_on_state;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
            }
        }

        service speed_limiter {
            let event clock: float = period(1000);
            let (signal v_set_aux: float, signal v_update: bool) = process_set_speed(set_speed);
            let (
                signal state: SpeedLimiter,
                signal on_state: SpeedLimiterOn,
                signal in_regulation_aux: bool,
                signal state_update: bool,
            ) = speed_limiter(
                activation,
                vacuum_brake,
                kickdown,
                vdc,
                speed,
                v_set,
                clock,
            );
            v_set = v_set_aux;
            in_regulation = in_regulation_aux;
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}
