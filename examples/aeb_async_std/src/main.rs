#![allow(warnings)]
mod aeb {
    use grust::grust;

    grust! {
        #![mode = demo,
           dump = "examples/aeb_async_std/out/mod.rs",
           spawn_with = "async_std::task::spawn",
           get_handle = "async_std::task::JoinHandle"]
        import signal car::speed_km_h                   : float;
        import event  car::detect::left::pedestrian_l   : float;
        import event  car::detect::right::pedestrian_r  : float;
        export signal car::urban::braking::brakes       : Braking;

        // Braking type
        enum Braking {
            NoBrake,
            SoftBrake,
            UrgentBrake,
        }

        // Formula: d = 2 * s^2 / (250 * f)
        // d = braking distance in metres (to be calculated).
        // s = speed in km/h.
        // 250 = fixed figure which is always used.
        // f = coefficient of friction, approx. 0.8 on dry asphalt.
        function compute_soft_braking_distance(speed: float) -> float {
            return speed * speed / 100.0;
        }

        // determine braking strategy
        function brakes(distance: float, speed: float) -> Braking {
            let braking_distance: float = compute_soft_braking_distance(speed);
            let response: Braking = if braking_distance < distance
                                    then Braking::SoftBrake
                                    else Braking::UrgentBrake;
            return response;
        }

        component braking_state(pedest: float?, timeout_pedestrian: unit?, speed: float)
                            -> (state: Braking)
            requires { 0. <= speed && speed < 55. } // urban limit
            ensures { when _x = pedest? => state != Braking::NoBrake } // safety
        {
            state = when {
                init                        => Braking::NoBrake,
                let d = pedest?             => brakes(d, speed),
                let _ = timeout_pedestrian? => Braking::NoBrake,
            };
        }

        service aeb @ [10, 3000] {
            let event pedestrian: float = merge(pedestrian_l, pedestrian_r);
            let event timeout_pedestrian: unit = timeout(pedestrian, 2000);
            brakes = braking_state(pedestrian, timeout_pedestrian, speed_km_h);
        }
    }
}

use aeb::{
    runtime::{RuntimeInit, RuntimeInput, RuntimeOutput},
    Braking,
};
use grust::{
    futures::{self, Stream, StreamExt},
    tokio,
};
use json::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// JSON input type, without timestamp.
#[derive(Deserialize, std::fmt::Debug)]
#[serde(tag = "variant", content = "value")]
pub enum Input {
    PedestrianL(f64),
    PedestrianR(f64),
    SpeedKmH(f64),
}
impl Input {
    fn into(self, instant: Instant) -> RuntimeInput {
        match self {
            Input::PedestrianL(val) => RuntimeInput::PedestrianL(val, instant),
            Input::PedestrianR(val) => RuntimeInput::PedestrianR(val, instant),
            Input::SpeedKmH(val) => RuntimeInput::SpeedKmH(val, instant),
        }
    }
}

/// JSON output type, without timestamp.
#[derive(Serialize, std::fmt::Debug)]
pub enum Output {
    Brakes(usize),
}
impl From<RuntimeOutput> for Output {
    fn from(value: RuntimeOutput) -> Self {
        match value {
            RuntimeOutput::Brakes(Braking::NoBrake, _) => Output::Brakes(0),
            RuntimeOutput::Brakes(Braking::SoftBrake, _) => Output::Brakes(1),
            RuntimeOutput::Brakes(Braking::UrgentBrake, _) => Output::Brakes(2),
        }
    }
}

#[async_std::main]
async fn main() {
    const INPUT_PATH: &str = "examples/aeb_async_std/data/inputs.json";
    const OUTPUT_PATH: &str = "examples/aeb_async_std/data/outputs.json";
    let INIT: Instant = Instant::now();

    // read inputs
    let read_stream = futures::stream::iter(read_json(INPUT_PATH));

    // transform in RuntimeInput + sleep
    let input_stream = read_stream.filter_map(move |input: Result<(u64, Input), _>| async move {
        match input {
            Ok((timestamp, input)) => {
                let duration = std::time::Duration::from_millis(timestamp as u64);
                let instant = INIT + duration;
                // sleep to model the arrival of inputs
                async_std::task::sleep(instant.duration_since(Instant::now())).await;
                Some(input.into(instant))
            }
            Err(_) => None,
        }
    });

    // initiate JSON file
    begin_json(OUTPUT_PATH);

    // collect N outputs
    const N: usize = 10;
    let (mut output_stream, handle) = aeb::run(INIT, input_stream, RuntimeInit { speed_km_h: 0.0 });
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
