#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum Activation {
    #[default]
    On,
    Off,
}
pub fn safety_distance(sv_v_m_s: f64, fv_v_m_s: f64) -> f64 {
    let sv_d_stop_m = (sv_v_m_s * 2.0f64) + ((sv_v_m_s * sv_v_m_s) / (2.0f64 * 5.886f64));
    let fv_d_stop_m = (fv_v_m_s * fv_v_m_s) / (2.0f64 * 5.886f64);
    sv_d_stop_m - fv_d_stop_m
}
pub struct CommandInput {
    pub distance_m: f64,
    pub sv_v_km_h: f64,
    pub t_ms: f64,
}
pub struct CommandOutput {
    pub brakes_command: f64,
}
pub struct CommandState {
    derive: core::time::derivation::DeriveState,
}
impl grust::core::Component for CommandState {
    type Input = CommandInput;
    type Output = CommandOutput;
    fn init() -> CommandState {
        CommandState {
            derive: <core::time::derivation::DeriveState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: CommandInput) -> CommandOutput {
        let comp_app_derive = {
            let core::time::derivation::DeriveOutput { i } =
                <core::time::derivation::DeriveState as grust::core::Component>::step(
                    &mut self.derive,
                    core::time::derivation::DeriveInput {
                        x: input.distance_m,
                        t: input.t_ms,
                    },
                );
            (i)
        };
        let distancing_m_s = comp_app_derive / 1000.0f64;
        let sv_v_m_s = input.sv_v_km_h / 3.6f64;
        let fv_v_m_s = sv_v_m_s + distancing_m_s;
        let d_safe_m = safety_distance(sv_v_m_s, fv_v_m_s);
        let brakes_command = (distancing_m_s * distancing_m_s) / (input.distance_m - d_safe_m);
        CommandOutput { brakes_command }
    }
}
pub struct ErrorInput {
    pub sv_v_km_h: f64,
    pub brakes_m_s_command: f64,
    pub t_ms: f64,
}
pub struct ErrorOutput {
    pub e_m_s: f64,
}
pub struct ErrorState {
    derive: core::time::derivation::DeriveState,
}
impl grust::core::Component for ErrorState {
    type Input = ErrorInput;
    type Output = ErrorOutput;
    fn init() -> ErrorState {
        ErrorState {
            derive: <core::time::derivation::DeriveState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: ErrorInput) -> ErrorOutput {
        let sv_v_m_s = input.sv_v_km_h / 3.6f64;
        let x = sv_v_m_s * 1000.0f64;
        let comp_app_derive = {
            let core::time::derivation::DeriveOutput { i } =
                <core::time::derivation::DeriveState as grust::core::Component>::step(
                    &mut self.derive,
                    core::time::derivation::DeriveInput {
                        x: x,
                        t: input.t_ms,
                    },
                );
            (i)
        };
        let a_m_s = comp_app_derive / (1000.0f64 * 1000.0f64);
        let a_m_s_command = -(input.brakes_m_s_command);
        let e_m_s = a_m_s_command - a_m_s;
        ErrorOutput { e_m_s }
    }
}
pub struct PidInput {
    pub sv_v_km_h: f64,
    pub b_m_s_command: f64,
    pub t_ms: f64,
}
pub struct PidOutput {
    pub b_m_s_control: f64,
}
pub struct PidState {
    error: ErrorState,
    backward_euler: core::time::integration::BackwardEulerState,
    derive: core::time::derivation::DeriveState,
}
impl grust::core::Component for PidState {
    type Input = PidInput;
    type Output = PidOutput;
    fn init() -> PidState {
        PidState {
            error: <ErrorState as grust::core::Component>::init(),
            backward_euler:
                <core::time::integration::BackwardEulerState as grust::core::Component>::init(),
            derive: <core::time::derivation::DeriveState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: PidInput) -> PidOutput {
        let p_e = {
            let ErrorOutput { e_m_s } = <ErrorState as grust::core::Component>::step(
                &mut self.error,
                ErrorInput {
                    sv_v_km_h: input.sv_v_km_h,
                    brakes_m_s_command: input.b_m_s_command,
                    t_ms: input.t_ms,
                },
            );
            (e_m_s)
        };
        let i_e = {
            let core::time::integration::BackwardEulerOutput { i } =
                <core::time::integration::BackwardEulerState as grust::core::Component>::step(
                    &mut self.backward_euler,
                    core::time::integration::BackwardEulerInput {
                        x: p_e,
                        t: input.t_ms,
                    },
                );
            (i)
        };
        let d_e = {
            let core::time::derivation::DeriveOutput { i } =
                <core::time::derivation::DeriveState as grust::core::Component>::step(
                    &mut self.derive,
                    core::time::derivation::DeriveInput {
                        x: p_e,
                        t: input.t_ms,
                    },
                );
            (i)
        };
        let b_m_s_control = ((1.0f64 * p_e) + (0.1f64 * i_e)) + (0.05f64 * d_e);
        PidOutput { b_m_s_control }
    }
}
pub struct ActivateInput {
    pub acc_active: Option<Activation>,
    pub change: Option<f64>,
    pub distance_m: f64,
}
pub struct ActivateOutput {
    pub condition: bool,
}
pub struct ActivateState {
    last_active: bool,
    last_approaching: bool,
    last_distance_m: f64,
}
impl grust::core::Component for ActivateState {
    type Input = ActivateInput;
    type Output = ActivateOutput;
    fn init() -> ActivateState {
        ActivateState {
            last_active: false,
            last_approaching: false,
            last_distance_m: 0.0f64,
        }
    }
    fn step(&mut self, input: ActivateInput) -> ActivateOutput {
        let (active, approaching) = match (input.acc_active, input.change) {
            (Some(acc_active), _) => {
                let approaching = self.last_approaching;
                let active = acc_active == Activation::On;
                (active, approaching)
            }
            (_, Some(change)) if input.distance_m < self.last_distance_m => {
                let active = self.last_active;
                let approaching = true;
                (active, approaching)
            }
            (_, Some(change)) if input.distance_m >= self.last_distance_m => {
                let active = self.last_active;
                let approaching = false;
                (active, approaching)
            }
            (_, _) => (self.last_active, self.last_approaching),
        };
        let condition = active && approaching;
        self.last_active = active;
        self.last_approaching = approaching;
        self.last_distance_m = input.distance_m;
        ActivateOutput { condition }
    }
}
pub struct FilteredAccInput {
    pub clock: Option<f64>,
    pub condition: bool,
    pub distance_m: f64,
    pub sv_v_km_h: f64,
    pub t_ms: f64,
}
pub struct FilteredAccOutput {
    pub brakes_m_s: f64,
}
pub struct FilteredAccState {
    last_brakes_command_m_s: f64,
    last_brakes_m_s: f64,
    last_x: bool,
    command: CommandState,
    pid: PidState,
}
impl grust::core::Component for FilteredAccState {
    type Input = FilteredAccInput;
    type Output = FilteredAccOutput;
    fn init() -> FilteredAccState {
        FilteredAccState {
            last_brakes_command_m_s: 0.0f64,
            last_brakes_m_s: 0.0f64,
            last_x: false,
            command: <CommandState as grust::core::Component>::init(),
            pid: <PidState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: FilteredAccInput) -> FilteredAccOutput {
        let x = !(input.condition);
        let (brakes_command_m_s, brakes_m_s) = match (input.clock) {
            (Some(clock)) if input.condition => {
                let brakes_command_m_s = {
                    let CommandOutput { brakes_command } =
                        <CommandState as grust::core::Component>::step(
                            &mut self.command,
                            CommandInput {
                                distance_m: input.distance_m,
                                sv_v_km_h: input.sv_v_km_h,
                                t_ms: input.t_ms,
                            },
                        );
                    (brakes_command)
                };
                let brakes_m_s = {
                    let PidOutput { b_m_s_control } = <PidState as grust::core::Component>::step(
                        &mut self.pid,
                        PidInput {
                            sv_v_km_h: input.sv_v_km_h,
                            b_m_s_command: brakes_command_m_s,
                            t_ms: input.t_ms,
                        },
                    );
                    (b_m_s_control)
                };
                (brakes_command_m_s, brakes_m_s)
            }
            (_) if x && !(self.last_x) => {
                let brakes_command_m_s = 0.0f64;
                let brakes_m_s = 0.0f64;
                (brakes_command_m_s, brakes_m_s)
            }
            (_) => (self.last_brakes_command_m_s, self.last_brakes_m_s),
        };
        self.last_brakes_command_m_s = brakes_command_m_s;
        self.last_brakes_m_s = brakes_m_s;
        self.last_x = x;
        FilteredAccOutput { brakes_m_s }
    }
}
pub mod runtime {
    use super::*;
    use grust::futures::{sink::SinkExt, stream::StreamExt};
    #[derive(Debug)]
    pub enum RuntimeInput {
        SpeedKmH(f64, std::time::Instant),
        DistanceM(f64, std::time::Instant),
        AccActive(Activation, std::time::Instant),
        Timer(T, std::time::Instant),
    }
    use RuntimeInput as I;
    impl grust::core::priority_stream::Reset for RuntimeInput {
        fn do_reset(&self) -> bool {
            match self {
                I::Timer(timer, _) => grust::core::timer_stream::Timing::do_reset(timer),
                _ => false,
            }
        }
    }
    impl PartialEq for RuntimeInput {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (I::SpeedKmH(this, _), I::SpeedKmH(other, _)) => this.eq(other),
                (I::DistanceM(this, _), I::DistanceM(other, _)) => this.eq(other),
                (I::AccActive(this, _), I::AccActive(other, _)) => this.eq(other),
                (I::Timer(this, _), I::Timer(other, _)) => this.eq(other),
                _ => false,
            }
        }
    }
    impl RuntimeInput {
        pub fn get_instant(&self) -> std::time::Instant {
            match self {
                I::SpeedKmH(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::DistanceM(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::AccActive(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Timer(_, _grust_reserved_instant) => *_grust_reserved_instant,
            }
        }
        pub fn order(v1: &Self, v2: &Self) -> std::cmp::Ordering {
            v1.get_instant().cmp(&v2.get_instant())
        }
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeOutput {
        BrakesMS(f64, std::time::Instant),
    }
    use RuntimeOutput as O;
    #[derive(Debug, Default)]
    pub struct RuntimeInit {
        pub speed_km_h: f64,
        pub distance_m: f64,
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeTimer {
        PeriodClock,
        DelayAdaptiveCruiseControl,
        TimeoutAdaptiveCruiseControl,
    }
    use RuntimeTimer as T;
    impl grust::core::timer_stream::Timing for RuntimeTimer {
        fn get_duration(&self) -> std::time::Duration {
            match self {
                T::PeriodClock => std::time::Duration::from_millis(1000u64),
                T::DelayAdaptiveCruiseControl => std::time::Duration::from_millis(10u64),
                T::TimeoutAdaptiveCruiseControl => std::time::Duration::from_millis(3000u64),
            }
        }
        fn do_reset(&self) -> bool {
            match self {
                T::PeriodClock => false,
                T::DelayAdaptiveCruiseControl => true,
                T::TimeoutAdaptiveCruiseControl => true,
            }
        }
    }
    pub struct Runtime {
        _grust_reserved_init_instant: std::time::Instant,
        adaptive_cruise_control: adaptive_cruise_control_service::AdaptiveCruiseControlService,
        output: grust::futures::channel::mpsc::Sender<O>,
        timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
    }
    impl Runtime {
        pub fn new(
            _grust_reserved_init_instant: std::time::Instant,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        ) -> Runtime {
            let adaptive_cruise_control =
                adaptive_cruise_control_service::AdaptiveCruiseControlService::init(
                    _grust_reserved_init_instant,
                    output.clone(),
                    timer.clone(),
                );
            Runtime {
                _grust_reserved_init_instant,
                adaptive_cruise_control,
                output,
                timer,
            }
        }
        #[inline]
        pub async fn send_timer(
            &mut self,
            timer: T,
            instant: std::time::Instant,
        ) -> Result<(), grust::futures::channel::mpsc::SendError> {
            self.timer.send((timer, instant)).await?;
            Ok(())
        }
        pub async fn run_loop(
            self,
            input: impl grust::futures::Stream<Item = I>,
            init_vals: RuntimeInit,
        ) -> Result<(), grust::futures::channel::mpsc::SendError> {
            grust::futures::pin_mut!(input);
            let mut runtime = self;
            let RuntimeInit {
                speed_km_h,
                distance_m,
            } = init_vals;
            runtime
                .adaptive_cruise_control
                .handle_init(speed_km_h, distance_m)
                .await?;
            while let Some(input) = input.next().await {
                match input {
                    I::Timer(T::DelayAdaptiveCruiseControl, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_delay_adaptive_cruise_control(_grust_reserved_instant)
                            .await?;
                    }
                    I::SpeedKmH(speed_km_h, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_speed_km_h(_grust_reserved_instant, speed_km_h)
                            .await?;
                    }
                    I::Timer(T::PeriodClock, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_period_clock(_grust_reserved_instant)
                            .await?;
                    }
                    I::DistanceM(distance_m, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_distance_m(_grust_reserved_instant, distance_m)
                            .await?;
                    }
                    I::AccActive(acc_active, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_acc_active(_grust_reserved_instant, acc_active)
                            .await?;
                    }
                    I::Timer(T::TimeoutAdaptiveCruiseControl, _grust_reserved_instant) => {
                        runtime
                            .adaptive_cruise_control
                            .handle_timeout_adaptive_cruise_control(_grust_reserved_instant)
                            .await?;
                    }
                }
            }
            Ok(())
        }
    }
    pub mod adaptive_cruise_control_service {
        use super::*;
        use grust::futures::{sink::SinkExt, stream::StreamExt};
        mod ctx_ty {
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct Condition(bool, bool);
            impl Condition {
                pub fn set(&mut self, condition: bool) {
                    self.1 = self.0 != condition;
                    self.0 = condition;
                }
                pub fn get(&self) -> bool {
                    self.0
                }
                pub fn take(&mut self) -> bool {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct SpeedKmH(f64, bool);
            impl SpeedKmH {
                pub fn set(&mut self, speed_km_h: f64) {
                    self.1 = self.0 != speed_km_h;
                    self.0 = speed_km_h;
                }
                pub fn get(&self) -> f64 {
                    self.0
                }
                pub fn take(&mut self) -> f64 {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct T(f64, bool);
            impl T {
                pub fn set(&mut self, t: f64) {
                    self.1 = self.0 != t;
                    self.0 = t;
                }
                pub fn get(&self) -> f64 {
                    self.0
                }
                pub fn take(&mut self) -> f64 {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct BrakesMS(f64, bool);
            impl BrakesMS {
                pub fn set(&mut self, brakes_m_s: f64) {
                    self.1 = self.0 != brakes_m_s;
                    self.0 = brakes_m_s;
                }
                pub fn get(&self) -> f64 {
                    self.0
                }
                pub fn take(&mut self) -> f64 {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct DistanceM(f64, bool);
            impl DistanceM {
                pub fn set(&mut self, distance_m: f64) {
                    self.1 = self.0 != distance_m;
                    self.0 = distance_m;
                }
                pub fn get(&self) -> f64 {
                    self.0
                }
                pub fn take(&mut self) -> f64 {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct XOld(f64, bool);
            impl XOld {
                pub fn set(&mut self, x_old: f64) {
                    self.1 = self.0 != x_old;
                    self.0 = x_old;
                }
                pub fn get(&self) -> f64 {
                    self.0
                }
                pub fn take(&mut self) -> f64 {
                    std::mem::take(&mut self.0)
                }
                pub fn is_new(&self) -> bool {
                    self.1
                }
                pub fn reset(&mut self) {
                    self.1 = false;
                }
            }
        }
        #[derive(Clone, Copy, PartialEq, Default, Debug)]
        pub struct Context {
            pub condition: ctx_ty::Condition,
            pub speed_km_h: ctx_ty::SpeedKmH,
            pub t: ctx_ty::T,
            pub brakes_m_s: ctx_ty::BrakesMS,
            pub distance_m: ctx_ty::DistanceM,
            pub x_old: ctx_ty::XOld,
        }
        impl Context {
            fn init() -> Context {
                Default::default()
            }
            fn reset(&mut self) {
                self.condition.reset();
                self.speed_km_h.reset();
                self.t.reset();
                self.brakes_m_s.reset();
                self.distance_m.reset();
                self.x_old.reset();
            }
        }
        #[derive(Default)]
        pub struct AdaptiveCruiseControlServiceStore {
            speed_km_h: Option<(f64, std::time::Instant)>,
            period_clock: Option<((), std::time::Instant)>,
            distance_m: Option<(f64, std::time::Instant)>,
            acc_active: Option<(Activation, std::time::Instant)>,
        }
        impl AdaptiveCruiseControlServiceStore {
            pub fn not_empty(&self) -> bool {
                self.speed_km_h.is_some()
                    || self.period_clock.is_some()
                    || self.distance_m.is_some()
                    || self.acc_active.is_some()
            }
        }
        pub struct AdaptiveCruiseControlService {
            _grust_reserved_init_instant: std::time::Instant,
            context: Context,
            delayed: bool,
            input_store: AdaptiveCruiseControlServiceStore,
            activate: ActivateState,
            filtered_acc: FilteredAccState,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        }
        impl AdaptiveCruiseControlService {
            pub fn init(
                _grust_reserved_init_instant: std::time::Instant,
                output: grust::futures::channel::mpsc::Sender<O>,
                timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
            ) -> AdaptiveCruiseControlService {
                let context = Context::init();
                let delayed = true;
                let input_store = Default::default();
                let activate = <ActivateState as grust::core::Component>::init();
                let filtered_acc = <FilteredAccState as grust::core::Component>::init();
                AdaptiveCruiseControlService {
                    _grust_reserved_init_instant,
                    context,
                    delayed,
                    input_store,
                    activate,
                    filtered_acc,
                    output,
                    timer,
                }
            }
            pub async fn handle_init(
                &mut self,
                speed_km_h: f64,
                distance_m: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                let _grust_reserved_instant = self._grust_reserved_init_instant;
                self.reset_service_timeout(_grust_reserved_instant).await?;
                let clock_ref = &mut None;
                self.context.distance_m.set(distance_m);
                self.context.x_old.set(distance_m);
                let ActivateOutput {
                    condition: condition,
                } = <ActivateState as grust::core::Component>::step(
                    &mut self.activate,
                    ActivateInput {
                        acc_active: None,
                        change: None,
                        distance_m: distance_m,
                    },
                );
                self.context.condition.set(condition);
                self.send_timer(T::PeriodClock, _grust_reserved_instant)
                    .await?;
                *clock_ref = Some(
                    (_grust_reserved_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64,
                );
                self.context.speed_km_h.set(speed_km_h);
                let t = (_grust_reserved_instant
                    .duration_since(self._grust_reserved_init_instant)
                    .as_millis()) as f64;
                self.context.t.set(t);
                let FilteredAccOutput {
                    brakes_m_s: brakes_m_s,
                } = <FilteredAccState as grust::core::Component>::step(
                    &mut self.filtered_acc,
                    FilteredAccInput {
                        clock: *clock_ref,
                        condition: self.context.condition.get(),
                        distance_m: distance_m,
                        sv_v_km_h: speed_km_h,
                        t_ms: t,
                    },
                );
                self.context.brakes_m_s.set(brakes_m_s);
                self.send_output(
                    O::BrakesMS(self.context.brakes_m_s.get(), _grust_reserved_instant),
                    _grust_reserved_instant,
                )
                .await?;
                Ok(())
            }
            pub async fn handle_speed_km_h(
                &mut self,
                _speed_km_h_instant: std::time::Instant,
                speed_km_h: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_speed_km_h_instant).await?;
                    self.context.reset();
                    self.context.speed_km_h.set(speed_km_h);
                    let t = (_speed_km_h_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64;
                    self.context.t.set(t);
                    if self.context.condition.is_new()
                        || self.context.distance_m.is_new()
                        || self.context.speed_km_h.is_new()
                        || self.context.t.is_new()
                    {
                        let FilteredAccOutput {
                            brakes_m_s: brakes_m_s,
                        } = <FilteredAccState as grust::core::Component>::step(
                            &mut self.filtered_acc,
                            FilteredAccInput {
                                clock: None,
                                condition: self.context.condition.get(),
                                distance_m: self.context.distance_m.get(),
                                sv_v_km_h: speed_km_h,
                                t_ms: t,
                            },
                        );
                        self.context.brakes_m_s.set(brakes_m_s);
                    }
                    if self.context.brakes_m_s.is_new() {
                        self.send_output(
                            O::BrakesMS(self.context.brakes_m_s.get(), _speed_km_h_instant),
                            _speed_km_h_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .speed_km_h
                        .replace((speed_km_h, _speed_km_h_instant));
                    assert ! (unique . is_none () , "flow `speed_km_h` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_period_clock(
                &mut self,
                _period_clock_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_period_clock_instant).await?;
                    self.context.reset();
                    let clock_ref = &mut None;
                    self.send_timer(T::PeriodClock, _period_clock_instant)
                        .await?;
                    *clock_ref = Some(
                        (_period_clock_instant
                            .duration_since(self._grust_reserved_init_instant)
                            .as_millis()) as f64,
                    );
                    let t = (_period_clock_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64;
                    self.context.t.set(t);
                    if clock_ref.is_some()
                        || self.context.condition.is_new()
                        || self.context.distance_m.is_new()
                        || self.context.speed_km_h.is_new()
                        || self.context.t.is_new()
                    {
                        let FilteredAccOutput {
                            brakes_m_s: brakes_m_s,
                        } = <FilteredAccState as grust::core::Component>::step(
                            &mut self.filtered_acc,
                            FilteredAccInput {
                                clock: *clock_ref,
                                condition: self.context.condition.get(),
                                distance_m: self.context.distance_m.get(),
                                sv_v_km_h: self.context.speed_km_h.get(),
                                t_ms: t,
                            },
                        );
                        self.context.brakes_m_s.set(brakes_m_s);
                    }
                    if self.context.brakes_m_s.is_new() {
                        self.send_output(
                            O::BrakesMS(self.context.brakes_m_s.get(), _period_clock_instant),
                            _period_clock_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .period_clock
                        .replace(((), _period_clock_instant));
                    assert ! (unique . is_none () , "flow `period_clock` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_distance_m(
                &mut self,
                _distance_m_instant: std::time::Instant,
                distance_m: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_distance_m_instant).await?;
                    self.context.reset();
                    let x_ref = &mut None;
                    self.context.distance_m.set(distance_m);
                    if self.context.x_old.get() != distance_m {
                        self.context.x_old.set(distance_m);
                        *x_ref = Some(distance_m);
                    }
                    if x_ref.is_some() || self.context.distance_m.is_new() {
                        let ActivateOutput {
                            condition: condition,
                        } = <ActivateState as grust::core::Component>::step(
                            &mut self.activate,
                            ActivateInput {
                                acc_active: None,
                                change: *x_ref,
                                distance_m: distance_m,
                            },
                        );
                        self.context.condition.set(condition);
                    }
                    let t = (_distance_m_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64;
                    self.context.t.set(t);
                    if self.context.condition.is_new()
                        || self.context.distance_m.is_new()
                        || self.context.speed_km_h.is_new()
                        || self.context.t.is_new()
                    {
                        let FilteredAccOutput {
                            brakes_m_s: brakes_m_s,
                        } = <FilteredAccState as grust::core::Component>::step(
                            &mut self.filtered_acc,
                            FilteredAccInput {
                                clock: None,
                                condition: self.context.condition.get(),
                                distance_m: distance_m,
                                sv_v_km_h: self.context.speed_km_h.get(),
                                t_ms: t,
                            },
                        );
                        self.context.brakes_m_s.set(brakes_m_s);
                    }
                    if self.context.brakes_m_s.is_new() {
                        self.send_output(
                            O::BrakesMS(self.context.brakes_m_s.get(), _distance_m_instant),
                            _distance_m_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .distance_m
                        .replace((distance_m, _distance_m_instant));
                    assert ! (unique . is_none () , "flow `distance_m` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_delay_adaptive_cruise_control(
                &mut self,
                _grust_reserved_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.context.reset();
                if self.input_store.not_empty() {
                    self.reset_time_constraints(_grust_reserved_instant).await?;
                    let period_clock_ref = &mut None;
                    let clock_ref = &mut None;
                    let x_ref = &mut None;
                    let acc_active_ref = &mut None;
                    let _acc_active_input_store = self.input_store.acc_active.take();
                    *acc_active_ref = _acc_active_input_store.map(|(x, _)| x);
                    let _distance_m_input_store = self.input_store.distance_m.take();
                    if let Some((distance_m, _)) = _distance_m_input_store {
                        self.context.distance_m.set(distance_m);
                    }
                    if self.context.x_old.get() != self.context.distance_m.get() {
                        self.context.x_old.set(self.context.distance_m.get());
                        *x_ref = Some(self.context.distance_m.get());
                    }
                    if acc_active_ref.is_some()
                        || x_ref.is_some()
                        || self.context.distance_m.is_new()
                    {
                        let ActivateOutput {
                            condition: condition,
                        } = <ActivateState as grust::core::Component>::step(
                            &mut self.activate,
                            ActivateInput {
                                acc_active: *acc_active_ref,
                                change: *x_ref,
                                distance_m: self.context.distance_m.get(),
                            },
                        );
                        self.context.condition.set(condition);
                    }
                    let _period_clock_input_store = self.input_store.period_clock.take();
                    if let Some((_, _period_clock_instant)) = _period_clock_input_store {
                        self.send_timer(T::PeriodClock, _period_clock_instant)
                            .await?;
                    }
                    *period_clock_ref = _period_clock_input_store.map(|(x, _)| x);
                    *clock_ref = _period_clock_input_store.map(|(_, y)| {
                        (y.duration_since(self._grust_reserved_init_instant)
                            .as_millis()) as f64
                    });
                    let _speed_km_h_input_store = self.input_store.speed_km_h.take();
                    if let Some((speed_km_h, _)) = _speed_km_h_input_store {
                        self.context.speed_km_h.set(speed_km_h);
                    }
                    let t = (_grust_reserved_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64;
                    self.context.t.set(t);
                    if clock_ref.is_some()
                        || self.context.condition.is_new()
                        || self.context.distance_m.is_new()
                        || self.context.speed_km_h.is_new()
                        || self.context.t.is_new()
                    {
                        let FilteredAccOutput {
                            brakes_m_s: brakes_m_s,
                        } = <FilteredAccState as grust::core::Component>::step(
                            &mut self.filtered_acc,
                            FilteredAccInput {
                                clock: *clock_ref,
                                condition: self.context.condition.get(),
                                distance_m: self.context.distance_m.get(),
                                sv_v_km_h: self.context.speed_km_h.get(),
                                t_ms: t,
                            },
                        );
                        self.context.brakes_m_s.set(brakes_m_s);
                    }
                    if self.context.brakes_m_s.is_new() {
                        self.send_output(
                            O::BrakesMS(self.context.brakes_m_s.get(), _grust_reserved_instant),
                            _grust_reserved_instant,
                        )
                        .await?;
                    }
                } else {
                    self.delayed = true;
                }
                Ok(())
            }
            #[inline]
            pub async fn reset_service_delay(
                &mut self,
                _grust_reserved_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.timer
                    .send((T::DelayAdaptiveCruiseControl, _grust_reserved_instant))
                    .await?;
                self.delayed = false;
                Ok(())
            }
            pub async fn handle_acc_active(
                &mut self,
                _acc_active_instant: std::time::Instant,
                acc_active: Activation,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_acc_active_instant).await?;
                    self.context.reset();
                    let acc_active_ref = &mut None;
                    *acc_active_ref = Some(acc_active);
                    if acc_active_ref.is_some() || self.context.distance_m.is_new() {
                        let ActivateOutput {
                            condition: condition,
                        } = <ActivateState as grust::core::Component>::step(
                            &mut self.activate,
                            ActivateInput {
                                acc_active: *acc_active_ref,
                                change: None,
                                distance_m: self.context.distance_m.get(),
                            },
                        );
                        self.context.condition.set(condition);
                    }
                    let t = (_acc_active_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64;
                    self.context.t.set(t);
                    if self.context.condition.is_new()
                        || self.context.distance_m.is_new()
                        || self.context.speed_km_h.is_new()
                        || self.context.t.is_new()
                    {
                        let FilteredAccOutput {
                            brakes_m_s: brakes_m_s,
                        } = <FilteredAccState as grust::core::Component>::step(
                            &mut self.filtered_acc,
                            FilteredAccInput {
                                clock: None,
                                condition: self.context.condition.get(),
                                distance_m: self.context.distance_m.get(),
                                sv_v_km_h: self.context.speed_km_h.get(),
                                t_ms: t,
                            },
                        );
                        self.context.brakes_m_s.set(brakes_m_s);
                    }
                    if self.context.brakes_m_s.is_new() {
                        self.send_output(
                            O::BrakesMS(self.context.brakes_m_s.get(), _acc_active_instant),
                            _acc_active_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .acc_active
                        .replace((acc_active, _acc_active_instant));
                    assert ! (unique . is_none () , "flow `acc_active` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_timeout_adaptive_cruise_control(
                &mut self,
                _timeout_adaptive_cruise_control_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.reset_time_constraints(_timeout_adaptive_cruise_control_instant)
                    .await?;
                self.context.reset();
                let t = (_timeout_adaptive_cruise_control_instant
                    .duration_since(self._grust_reserved_init_instant)
                    .as_millis()) as f64;
                self.context.t.set(t);
                if self.context.condition.is_new()
                    || self.context.distance_m.is_new()
                    || self.context.speed_km_h.is_new()
                    || self.context.t.is_new()
                {
                    let FilteredAccOutput {
                        brakes_m_s: brakes_m_s,
                    } = <FilteredAccState as grust::core::Component>::step(
                        &mut self.filtered_acc,
                        FilteredAccInput {
                            clock: None,
                            condition: self.context.condition.get(),
                            distance_m: self.context.distance_m.get(),
                            sv_v_km_h: self.context.speed_km_h.get(),
                            t_ms: t,
                        },
                    );
                    self.context.brakes_m_s.set(brakes_m_s);
                }
                self.send_output(
                    O::BrakesMS(
                        self.context.brakes_m_s.get(),
                        _timeout_adaptive_cruise_control_instant,
                    ),
                    _timeout_adaptive_cruise_control_instant,
                )
                .await?;
                Ok(())
            }
            #[inline]
            pub async fn reset_service_timeout(
                &mut self,
                _timeout_adaptive_cruise_control_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.timer
                    .send((
                        T::TimeoutAdaptiveCruiseControl,
                        _timeout_adaptive_cruise_control_instant,
                    ))
                    .await?;
                Ok(())
            }
            #[inline]
            pub async fn reset_time_constraints(
                &mut self,
                instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.reset_service_delay(instant).await?;
                Ok(())
            }
            #[inline]
            pub async fn send_output(
                &mut self,
                output: O,
                instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.reset_service_timeout(instant).await?;
                self.output.feed(output).await?;
                Ok(())
            }
            #[inline]
            pub async fn send_timer(
                &mut self,
                timer: T,
                instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.timer.feed((timer, instant)).await?;
                Ok(())
            }
        }
    }
}
use grust::futures::{Stream, StreamExt};
pub fn run(
    _grust_reserved_init_instant: std::time::Instant,
    input_stream: impl Stream<Item = runtime::RuntimeInput> + Send + 'static,
    init_signals: runtime::RuntimeInit,
) -> grust::futures::channel::mpsc::Receiver<runtime::RuntimeOutput> {
    const TIMER_CHANNEL_SIZE: usize = 3usize + 2;
    const TIMER_STREAM_SIZE: usize = 3usize + 2;
    let (timers_sink, timers_stream) = grust::futures::channel::mpsc::channel(TIMER_CHANNEL_SIZE);
    let timers_stream =
        grust::core::timer_stream::timer_stream::<_, _, TIMER_STREAM_SIZE>(timers_stream)
            .map(|(timer, deadline)| runtime::RuntimeInput::Timer(timer, deadline));
    const OUTPUT_CHANNEL_SIZE: usize = 1usize;
    let (output_sink, output_stream) = grust::futures::channel::mpsc::channel(OUTPUT_CHANNEL_SIZE);
    const PRIO_STREAM_SIZE: usize = 4usize;
    let prio_stream = grust::core::priority_stream::prio_stream::<_, _, PRIO_STREAM_SIZE>(
        grust::futures::stream::select(input_stream, timers_stream),
        runtime::RuntimeInput::order,
    );
    let service = runtime::Runtime::new(_grust_reserved_init_instant, output_sink, timers_sink);
    grust::tokio::spawn(async move {
        let result = service.run_loop(prio_stream, init_signals).await;
        assert!(result.is_ok())
    });
    output_stream
}
