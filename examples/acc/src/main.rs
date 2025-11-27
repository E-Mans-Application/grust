#![allow(warnings)]

use grust::grust;

pub mod utils {
    pub fn convert(x_km_h: f64) -> f64 {
        x_km_h / 3.6
    }
}

grust! {
    #![mode = demo, dump = "examples/acc/out/mod.rs", public = false]
    use function utils::convert(x_km_h: float) -> float;

    // Interface to the broader car system
    import signal car::state::speed_km_h        : float;
    import signal car::sensors::radar_m         : float;
    import event  car::hmi::acc_active          : Activation;
    export signal car::actuators::brakes_m_s    : float;
    import event car::core::derive              : unit;

    // Activation type
    enum Activation{ On, Off }

    const MIN: int = 10;
    const MAX: int = 3000;
    const RHO: float = 1.;         // reaction time
    const B_MAX: float = 0.6*9.81; // max braking

    // Adaptive Cruise Control service
    service adaptive_cruise_control @[MIN, MAX] {
        let event  radar_e: float = on_change(radar_m);
        let signal cond: bool = activate(acc_active, radar_e);
        let signal speed_m_s: float = convert(speed_km_h);
        let signal delta_v: float = derive_on(radar_m, time(), derive);
        brakes_m_s = acc(cond, radar_m, speed_m_s, delta_v);
    }

    // Filters the ACC on driver activation and when approaching FV
    component acc(c: bool, d: float, s: float, v: float) -> (b: float) {
        log (c, d, s, v, b, d_safe, fv_v);
        match c {
            true => {
                b = v^2 / (2. * (d - d_safe));
                let d_safe: float = safety_distance(s, fv_v);
                let fv_v: float = s + v;
            }
            false => {
                b = 0.;
                let (fv_v: float, d_safe: float) = (0., 0.);
            }
        }
    }

    // Activation condition of the ACC
    component activate(act: Activation?, r: float?) -> (c: bool) {
        log (act, r, r_mem, active, approach, c);
        when {
            init => { r_mem = 0.; active = false; approach = false; }
            act? => { let active: bool = act == Activation::On; }
            r?   => { let r_mem: float = r; let approach: bool = r < last r_mem; }
        }
        c = active && approach;
    }
    // Derive on a specific event ‘e‘.
    component derive_on(x: float, t: float, e: unit?) -> (v: float) {
        log (t, e, v, x_mem);
        when {
            init => { v = 0.; x_mem = 0.; t_mem = 0.; }
            e? => {
                v = 1000. * (x - last x_mem) / (t - last t_mem);
                let (x_mem: float, t_mem: float) = (x, t);
            }
        }
    }

    // Safety distance computation
    function safety_distance(sv_v: float, fv_v: float) -> float {
        let sv_d_stop: float = sv_v*RHO + sv_v^2/(2.*B_MAX);
        let fv_d_stop: float = fv_v^2/(2.*B_MAX);
        return sv_d_stop - fv_d_stop;
    }
}

use grust::{futures, tokio};
use json::*;
use runtime::{RuntimeInit, RuntimeInput, RuntimeOutput};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// JSON input type, without timestamp.
#[derive(Deserialize, Debug)]
#[serde(tag = "variant", content = "value")]
pub enum Input {
    SpeedKmH(f64),
    RadarM(f64),
    AccActive(usize), // Activation { On -> 0, Off -> 1 }
    Derive,
}
impl From<(Instant, Input)> for RuntimeInput {
    fn from(value: (Instant, Input)) -> Self {
        match value {
            (instant, Input::SpeedKmH(val)) => RuntimeInput::SpeedKmH(val, instant),
            (instant, Input::RadarM(val)) => RuntimeInput::RadarM(val, instant),
            (instant, Input::AccActive(0)) => RuntimeInput::AccActive(Activation::On, instant),
            (instant, Input::AccActive(1)) => RuntimeInput::AccActive(Activation::Off, instant),
            (instant, Input::Derive) => RuntimeInput::Derive((), instant),
            _ => panic!("{value:?} is not an input"),
        }
    }
}

/// JSON output type, without timestamp.
#[derive(Serialize, Debug)]
pub enum Output {
    BrakesMS(f64),
}
#[derive(Serialize, Debug)]
pub struct TimedOutput(u64, Output);
impl From<(Instant, RuntimeOutput)> for TimedOutput {
    fn from(value: (Instant, RuntimeOutput)) -> Self {
        match value {
            (INIT, RuntimeOutput::BrakesMS(val, instant)) => TimedOutput(
                instant.duration_since(INIT).as_millis() as u64,
                Output::BrakesMS(val),
            ),
        }
    }
}

#[tokio::main]
async fn main() {
    const INPUT_PATH: &str = "examples/acc/data/inputs.json";
    const OUTPUT_PATH: &str = "examples/acc/data/outputs.json";
    let INIT: Instant = Instant::now();

    // read inputs
    let read_stream = futures::stream::iter(read_json(INPUT_PATH));

    // transform in RuntimeInput + sleep
    let input_stream = read_stream.filter_map(move |input: Result<(u64, Input), _>| async move {
        match input {
            Ok((timestamp, input)) => {
                println!("\tinput: {input:?}");
                let duration = tokio::time::Duration::from_millis(timestamp as u64);
                let instant = INIT + duration;
                // sleep to model the arrival of inputs
                tokio::time::sleep_until(instant.into()).await;
                Some((instant, input).into())
            }
            Err(err) => panic!("{err}"),
        }
    });

    // initiate JSON file
    begin_json(OUTPUT_PATH);

    // collect N outputs
    const N: usize = 10;
    let mut output_stream = run(
        INIT,
        input_stream,
        RuntimeInit {
            speed_km_h: 129.,
            radar_m: 128.,
        },
    );
    let mut counter = 0;
    while let Some(received) = output_stream.next().await {
        counter += 1;
        let output: TimedOutput = (INIT, received).into();
        append_json(OUTPUT_PATH, output);
        // stop at N
        if counter >= N {
            break;
        }
    }

    // finalize JSON file
    end_json(OUTPUT_PATH);
}
