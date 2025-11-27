#![allow(warnings)]
mod sl {
    use grust::grust;

    grust! {
        #![mode = demo, dump = "examples/speed_limiter/out/mod.rs"]

        // # Imports
        import event    car::hmi::speed_limiter::activation : ActivationRequest;
        import signal   car::hmi::speed_limiter::set_speed  : float;
        import signal   car::adas::speed                    : float;
        import signal   car::adas::vacuum_brake             : VacuumBrakeState;
        import event    car::adas::kickdown                 : Kickdown;
        import event    car::adas::failure                  : Failure;

        export event    car::adas::speed_limiter::in_regulation : bool;
        export signal   car::adas::speed_limiter::v_set         : float;
        export signal   car::adas::speed_limiter::sl_state      : SpeedLimiterOn;

        // # Types

        // Hysterisis for speed.
        struct Hysterisis {
            value: float,
            flag: bool,
        }
        function new_hysterisis(value: float) -> Hysterisis {
            return Hysterisis { value: value, flag: true };
        }

        // Enumerates the kinds of activation requests.
        enum ActivationRequest { Off, On }

        // Vehicle dynamic control states.
        enum VdcState { On, Off }

        // Vacuum brake states.
        enum VacuumBrakeState { BelowMinLevel, AboveMinLevel }

        // Tells if the driver presses down hard on the accelerator or not.
        enum Kickdown { Deactivated, Activated }

        enum Failure { Entering, Recovered }

        // Speed limiter states.
        enum SpeedLimiter {
            Off,
            On,
            Fail,
        }

        // Speed limiter 'On' states.
        enum SpeedLimiterOn {
            StandBy,
            Active,
            OverrideVoluntary,
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

        // Speed limiter 'Activation' condition.
        function activation_condition(vacuum_brake_state: VacuumBrakeState, v_set: float) -> bool {
            return vacuum_brake_state != VacuumBrakeState::BelowMinLevel && v_set > 0.0;
        }

        // Speed limiter 'StandBy' condition.
        function standby_condition(vacuum_brake_state: VacuumBrakeState, v_set: float) -> bool {
            return vacuum_brake_state == VacuumBrakeState::BelowMinLevel || v_set <= 0.0;
        }

        // # Components

        // Processes the speed set by the driver.
        component process_set_speed(set_speed: float?) -> (v_set: float, v_update: bool) {
            let prev_v_set: float = last v_set;
            v_update = prev_v_set != v_set;
            v_set = when {
                init                => 0.,
                let v = set_speed?  => threshold_set_speed(v)
            };
        }

        // Speed limiter state machine.
        component speed_limiter(
            activation_req: ActivationRequest?,
            vacuum_brake_state: VacuumBrakeState,
            kickdown: Kickdown?,
            failure: Failure?,
            speed: float,
            v_set: float,
            clock: float?,
        ) -> (
            state: SpeedLimiter,
            on_state: SpeedLimiterOn,
            in_regulation: bool,
            state_update: bool,
        ) {
            let prev_state: SpeedLimiter = last state;
            let prev_on_state: SpeedLimiterOn = last on_state;
            state = when {
                init => SpeedLimiter::Off,
                activation_req? if activation_req == ActivationRequest::Off => SpeedLimiter::Off,
                let ActivationRequest::On = activation_req? if prev_state == SpeedLimiter::Off => SpeedLimiter::On,
                failure? if failure == Failure::Entering => SpeedLimiter::Fail,
                let f = failure? if f == Failure::Recovered && prev_state == SpeedLimiter::Fail => SpeedLimiter::On,
            };
            when {
                init => {
                    on_state = SpeedLimiterOn::StandBy;
                    in_regulation = true;
                    state_update = true;
                }
                clock? => {
                    match prev_state {
                        SpeedLimiter::On => {
                            (on_state, in_regulation, state_update) = speed_limiter_on(
                                prev_on_state,
                                vacuum_brake_state,
                                kickdown, speed, v_set,
                            );
                        },
                        _ => {
                            on_state = SpeedLimiterOn::StandBy;
                            in_regulation = false;
                            state_update = prev_state != state;
                        },
                    }
                }
            }
        }

        // Speed limiter 'On' state machine.
        component speed_limiter_on(
            prev_on_state: SpeedLimiterOn,
            vacuum_brake_state: VacuumBrakeState,
            kickdown: Kickdown?,
            speed: float,
            v_set: float,
        ) -> (
            on_state: SpeedLimiterOn,
            in_reg: bool,
            state_update: bool,
        ) {
            state_update = prev_on_state != on_state;
            init hysterisis = new_hysterisis(0.0);
            let prev_hysterisis: Hysterisis = last hysterisis;
            in_reg = in_regulation(hysterisis);
            let kickdown_state: Kickdown = when {
                init => Kickdown::Deactivated,
                let Kickdown::Activated = kickdown? if prev_on_state == SpeedLimiterOn::Active => Kickdown::Activated,
                let Kickdown::Deactivated = kickdown? => Kickdown::Deactivated,
            };
            match prev_on_state {
                _ if kickdown_state == Kickdown::Activated => {
                    on_state = SpeedLimiterOn::OverrideVoluntary;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
                SpeedLimiterOn::StandBy if activation_condition(vacuum_brake_state, v_set) => {
                    on_state = SpeedLimiterOn::Active;
                    let hysterisis: Hysterisis = new_hysterisis(0.0);
                },
                SpeedLimiterOn::OverrideVoluntary if speed <= v_set => {
                    on_state = SpeedLimiterOn::Active;
                    let hysterisis: Hysterisis = new_hysterisis(0.0);
                },
                SpeedLimiterOn::Active if standby_condition(vacuum_brake_state, v_set) => {
                    on_state = SpeedLimiterOn::StandBy;
                    let hysterisis: Hysterisis = prev_hysterisis;
                },
                SpeedLimiterOn::Active => {
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
            let event changed_set_speed: float = on_change(throttle(set_speed, 1.0));
            let event clock: float = period(1000);

            let (signal v_set_aux: float, signal v_update: bool) = process_set_speed(changed_set_speed);
            let (
                signal state: SpeedLimiter,
                signal on_state: SpeedLimiterOn,
                signal in_regulation_aux: bool,
                signal state_update: bool,
            ) = speed_limiter(
                activation,
                vacuum_brake,
                kickdown,
                failure,
                speed,
                v_set,
                clock,
            );
            v_set = v_set_aux;
            in_regulation = on_change(in_regulation_aux);
            sl_state = on_state;
        }
    }
}

use grust::{
    futures::{self, Stream, StreamExt},
    tokio,
};
use json::*;
use serde::{Deserialize, Serialize};
use sl::{
    runtime::{RuntimeInit, RuntimeInput, RuntimeOutput},
    ActivationRequest, Failure, Kickdown, SpeedLimiterOn, VacuumBrakeState, VdcState,
};
use std::time::{Duration, Instant};

/// JSON input type, without timestamp.
#[derive(Deserialize, std::fmt::Debug)]
#[serde(tag = "variant", content = "value")]
pub enum Input {
    Activation(bool),
    SetSpeed(f64),
    Speed(f64),
    VacuumBrake(bool),
    Kickdown(bool),
    Failure(bool),
}
impl Input {
    fn into(self, instant: Instant) -> RuntimeInput {
        match self {
            Input::Activation(true) => RuntimeInput::Activation(ActivationRequest::On, instant),
            Input::Activation(false) => RuntimeInput::Activation(ActivationRequest::Off, instant),
            Input::SetSpeed(set_speed) => RuntimeInput::SetSpeed(set_speed, instant),
            Input::Speed(speed) => RuntimeInput::Speed(speed, instant),
            Input::VacuumBrake(true) => {
                RuntimeInput::VacuumBrake(VacuumBrakeState::BelowMinLevel, instant)
            }
            Input::VacuumBrake(false) => {
                RuntimeInput::VacuumBrake(VacuumBrakeState::AboveMinLevel, instant)
            }
            Input::Kickdown(true) => RuntimeInput::Kickdown(Kickdown::Activated, instant),
            Input::Kickdown(false) => RuntimeInput::Kickdown(Kickdown::Deactivated, instant),
            Input::Failure(true) => RuntimeInput::Failure(Failure::Recovered, instant),
            Input::Failure(false) => RuntimeInput::Failure(Failure::Entering, instant),
        }
    }
}

/// JSON output type, without timestamp.
#[derive(Serialize, std::fmt::Debug)]
pub enum Output {
    InRegulation(bool),
    VSet(f64),
    SlState(usize),
}
impl From<RuntimeOutput> for Output {
    fn from(value: RuntimeOutput) -> Self {
        match value {
            RuntimeOutput::InRegulation(in_regulation, _) => Output::InRegulation(in_regulation),
            RuntimeOutput::VSet(v_set, _) => Output::VSet(v_set),
            RuntimeOutput::SlState(SpeedLimiterOn::StandBy, _) => Output::SlState(2),
            RuntimeOutput::SlState(SpeedLimiterOn::Active, _) => Output::SlState(3),
            RuntimeOutput::SlState(SpeedLimiterOn::OverrideVoluntary, _) => Output::SlState(4),
        }
    }
}

#[tokio::main]
async fn main() {
    const INPUT_PATH: &str = "examples/speed_limiter/data/inputs.json";
    const OUTPUT_PATH: &str = "examples/speed_limiter/data/outputs.json";
    let INIT: Instant = Instant::now();

    // read inputs
    let read_stream = futures::stream::iter(read_json(INPUT_PATH));

    // transform in RuntimeInput + sleep
    let input_stream = read_stream.filter_map(move |input: Result<(u64, Input), _>| async move {
        match input {
            Ok((timestamp, input)) => {
                let duration = tokio::time::Duration::from_millis(timestamp as u64);
                let instant = INIT + duration;
                // sleep to model the arrival of inputs
                tokio::time::sleep_until(instant.into()).await;
                Some(input.into(instant))
            }
            Err(_) => None,
        }
    });

    // initiate JSON file
    begin_json(OUTPUT_PATH);

    // collect N outputs
    const N: usize = 10;
    let mut output_stream = sl::run(
        INIT,
        input_stream,
        RuntimeInit {
            vacuum_brake: VacuumBrakeState::BelowMinLevel,
            set_speed: 0.0,
            speed: 0.0,
        },
    );
    let mut counter = 0;
    while let Some(received) = output_stream.next().await {
        counter += 1;
        let output: Output = received.into();
        append_json(OUTPUT_PATH, output);
        // stop at N
        if counter >= N {
            break;
        }
    }

    // finalize JSON file
    end_json(OUTPUT_PATH);
}
