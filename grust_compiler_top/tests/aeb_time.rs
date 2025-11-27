grust_compiler_top::prelude! {}

#[test]
fn should_compile_aeb_time() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/aeb_time.rs", mode = demo]

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
        function compute_soft_braking_distance(speed: float, acc: float) -> float {
            return speed * speed / (100.0 * acc);
        }

        // determine braking strategy
        function brakes(distance: float, speed: float, acc: float) -> Braking {
            let braking_distance: float = compute_soft_braking_distance(speed, acc);
            let response: Braking = if braking_distance < distance
                                    then Braking::SoftBrake
                                    else Braking::UrgentBrake;
            return response;
        }

        component derive_on(v_km_h: float, clock: float?) -> (a_km_h: float) {
            init v = 0.;
            let v: float = v_km_h / 3.6;
            when {
                init => { a = 0.; mem_t = 0.; }
                let t = clock? => {
                    let a: float = (v - last v)/(mem_t - last mem_t);
                    let mem_t: float = t;
                }
            }
            a_km_h = 3.6 * a;
        }

        component braking_state(pedest: float?, timeout_pedest: unit?, speed: float, acc: float) -> (state: Braking)
            requires { 0. <= speed && speed < 55. } // urban limit
            ensures { when _x = pedest? => state != Braking::NoBrake } // safety
        {
            when {
                init => {
                    state = Braking::NoBrake;
                }
                let d = pedest? => {
                    state = brakes(d, speed, acc);
                }
                let _ = timeout_pedest? => {
                    state = Braking::NoBrake;
                }
            }
        }

        service aeb @ [10, 3000] {
            let event pedestrian: float = merge(pedestrian_l, pedestrian_r);
            let event timeout_pedest: unit = timeout(pedestrian, 2000);
            let signal acc_km_h: float = derive_on(speed_km_h, period(10));
            brakes = braking_state(pedestrian, timeout_pedest, speed_km_h, acc_km_h);
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}
