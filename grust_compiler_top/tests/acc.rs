grust_compiler_top::prelude! {}

#[test]
fn should_compile_acc() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/acc.rs", mode = demo]

        use component core::time::integration::backward_euler(x: float, t: float) -> (i: float);
        use component core::time::derivation::derive(x: float, t: float) -> (i: float);

        // Vehicle speed, computed by another service.
        import signal car::state::speed_km_h                : float;
        // Front distance, from radar sensor.
        import signal car::sensors::radar::distance_m       : float;
        // Activation status by the driver.
        import event  car::cluster::acc_active              : Activation;
        // Braking to reach to maintain safety distance.
        export signal car::actuators::control::brakes_m_s   : float;

        // Activation type.
        enum Activation{ On, Off }

        const RHO: float = 2.0; // reaction time
        const B_MAX: float = 5.886; // 0.6*9.81

        // Safety distance computation.
        function safety_distance(sv_v_m_s: float, fv_v_m_s: float) -> float {
            // distance for SV to stop if it brakes max after a reaction time `RHO`
            let sv_d_stop_m: float = sv_v_m_s*RHO + sv_v_m_s*sv_v_m_s/(2.*B_MAX);
            // distance for FV to stop if it brakes max
            let fv_d_stop_m: float = fv_v_m_s*fv_v_m_s/(2.*B_MAX);
            return sv_d_stop_m - fv_d_stop_m;
        }

        // Command maintaining safety distance.
        //
        // If SV is getting closer to FV, then we need to maintain a safety distance `d_safe`:
        // => find `b_c` such that
        //      ->   sv_v - b_c*t = fv_v
        //      ->   distance(t) = fv_x + fv_v*t - (sv_x + sv_v*t - b_c*t²/2) > d_safe
        // => b_c > (fv_v - sv_v)²/(fv_x - sv_x - d_safe)
        component command(distance_m: float, sv_v_km_h: float, t_ms: float) -> (brakes_command: float) {
            let distancing_m_s: float = derive(distance_m, t_ms) / 1000.;
            brakes_command = distancing_m_s*distancing_m_s / (distance_m - d_safe_m);
            let d_safe_m: float = safety_distance(sv_v_m_s, fv_v_m_s);
            let fv_v_m_s: float = sv_v_m_s + distancing_m_s;
            let sv_v_m_s: float = sv_v_km_h / 3.6;
        }

        // Error on command.
        component error(sv_v_km_h: float, brakes_m_s_command: float, t_ms: float) -> (e_m_s: float) {
            let a_m_s: float = (derive(sv_v_m_s * 1000., t_ms)) / (1000.^2);
            let sv_v_m_s: float = sv_v_km_h / 3.6;
            e_m_s = a_m_s_command - a_m_s;
            let a_m_s_command: float = -brakes_m_s_command;
        }

        // Proportional Integral Derivative controller.
        component pid(sv_v_km_h: float, b_m_s_command: float, t_ms: float) -> (b_m_s_control: float) {
            let p_e: float = error(sv_v_km_h, b_m_s_command, t_ms); // units???
            let i_e: float = backward_euler(p_e, t_ms);
            let d_e: float = derive(p_e, t_ms);
            b_m_s_control = 1.*p_e + 0.1*i_e + 0.05*d_e;
        }

        // Returns the activation condition of the ACC.
        component activate(acc_active: Activation?, change: float?, distance_m: float) -> (condition: bool) {
            init distance_m = 0.;
            when {
                init => {
                    active = false;
                    approaching = false;
                }
                acc_active? => {
                    let active: bool = acc_active == Activation::On;
                }
                change? if distance_m < last distance_m => {
                    let approaching: bool = true;
                }
                change? if distance_m >= last distance_m => {
                    let approaching: bool = false;
                }
            }
            condition = active && approaching;
        }

        // Filters the ACC to activated only when activated and our vehicle approaches the front vehicle.
        component filtered_acc(clock: float?, condition: bool, distance_m: float, sv_v_km_h: float, t_ms: float) -> (brakes_m_s: float) {
            when {
                init => {
                    brakes_command_m_s = 0.;
                    brakes_m_s = 0.;
                }
                clock? if condition => {
                    let brakes_command_m_s: float = command(distance_m, sv_v_km_h, t_ms);
                    brakes_m_s = pid(sv_v_km_h, brakes_command_m_s, t_ms);
                }
                !condition => {
                    let brakes_command_m_s: float = 0.;
                    brakes_m_s = 0.;
                }
            }
        }

        // Adaptive Cruise Control
        //
        // This service computes the braking acceleration to perform
        // in order to maintain a safety distance `d_safe` between
        // the subject vehicle (SV) that we control and the front vehicle (FV).
        // In the example bellow, the total `distance` is the sum of `d_safe`
        // and a grace distance `d_grace`.
        //
        //  SV   d_grace       d_safe       FV
        //   x <---------><---------------> x
        //
        // Our goal is to keep `d_grace` above zero, by controlling the brakes.
        service adaptive_cruise_control @ [10, 3000] {
            let signal t: float = time();
            let signal condition: bool = activate(acc_active, on_change(distance_m), distance_m);
            let event clock: float = period(1000);
            brakes_m_s = filtered_acc(clock, condition, distance_m, speed_km_h, t);
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}
