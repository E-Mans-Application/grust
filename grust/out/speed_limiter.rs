#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub struct Hysterisis {
    pub value: f64,
    pub flag: bool,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum ActivationRequest {
    #[default]
    Off,
    On,
    Initialization,
    StandBy,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum VdcState {
    #[default]
    On,
    Off,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum VacuumBrakeState {
    #[default]
    BelowMinLevel,
    AboveMinLevel,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum KickdownState {
    #[default]
    Deactivated,
    Activated,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum SpeedLimiter {
    #[default]
    Off,
    On,
    Fail,
}
#[derive(Clone, Copy, PartialEq, Default, Debug)]
pub enum SpeedLimiterOn {
    #[default]
    StandBy,
    Actif,
    OverrideVoluntary,
    OverrideInvoluntary,
}
pub fn new_hysterisis(value: f64) -> Hysterisis {
    Hysterisis {
        value: value,
        flag: true,
    }
}
pub fn update_hysterisis(prev_hyst: Hysterisis, speed: f64, v_set: f64) -> Hysterisis {
    let activation_threshold = v_set * 0.99f64;
    let deactivation_threshold = v_set * 0.98f64;
    let flag = if prev_hyst.flag && (speed <= deactivation_threshold) {
        false
    } else {
        if !(prev_hyst.flag) && (speed >= activation_threshold) {
            true
        } else {
            prev_hyst.flag
        }
    };
    let new_hysterisis = Hysterisis {
        value: speed,
        flag: flag,
    };
    new_hysterisis
}
pub fn in_regulation(hysterisis: Hysterisis) -> bool {
    hysterisis.flag
}
pub fn into_diagnostic(to_be_defined: i64) -> i64 {
    to_be_defined
}
pub fn threshold_set_speed(set_speed: f64) -> f64 {
    let ceiled_speed = if set_speed > 150.0f64 {
        150.0f64
    } else {
        set_speed
    };
    let grounded_speed = if set_speed < 10.0f64 {
        10.0f64
    } else {
        ceiled_speed
    };
    grounded_speed
}
pub fn off_condition(activation_req: ActivationRequest, vdc_disabled: VdcState) -> bool {
    (activation_req == ActivationRequest::Off) || (vdc_disabled == VdcState::Off)
}
pub fn on_condition(activation_req: ActivationRequest) -> bool {
    (activation_req == ActivationRequest::On)
        || (activation_req == ActivationRequest::Initialization)
}
pub fn activation_condition(
    activation_req: ActivationRequest,
    vacuum_brake_state: VacuumBrakeState,
    v_set: f64,
) -> bool {
    ((activation_req == ActivationRequest::On)
        && (vacuum_brake_state != VacuumBrakeState::BelowMinLevel))
        && (v_set > 0.0f64)
}
pub fn exit_override_condition(
    activation_req: ActivationRequest,
    kickdown: KickdownState,
    v_set: f64,
    speed: f64,
) -> bool {
    (on_condition(activation_req) && (kickdown != KickdownState::Activated)) && (speed <= v_set)
}
pub fn involuntary_override_condition(
    activation_req: ActivationRequest,
    kickdown: KickdownState,
    v_set: f64,
    speed: f64,
) -> bool {
    (on_condition(activation_req) && (kickdown != KickdownState::Activated)) && (speed > v_set)
}
pub fn voluntary_override_condition(
    activation_req: ActivationRequest,
    kickdown: KickdownState,
) -> bool {
    on_condition(activation_req) && (kickdown == KickdownState::Activated)
}
pub fn standby_condition(
    activation_req: ActivationRequest,
    vacuum_brake_state: VacuumBrakeState,
    v_set: f64,
) -> bool {
    ((activation_req == ActivationRequest::StandBy)
        || (vacuum_brake_state == VacuumBrakeState::BelowMinLevel))
        || (v_set <= 0.0f64)
}
pub struct ProcessSetSpeedInput {
    pub set_speed: f64,
}
pub struct ProcessSetSpeedOutput {
    pub v_set: f64,
    pub v_update: bool,
}
pub struct ProcessSetSpeedState {
    last_v_set: f64,
}
impl grust::core::Component for ProcessSetSpeedState {
    type Input = ProcessSetSpeedInput;
    type Output = ProcessSetSpeedOutput;
    fn init() -> ProcessSetSpeedState {
        ProcessSetSpeedState { last_v_set: 0.0f64 }
    }
    fn step(&mut self, input: ProcessSetSpeedInput) -> ProcessSetSpeedOutput {
        let v_set = threshold_set_speed(input.set_speed);
        let prev_v_set = self.last_v_set;
        let v_update = prev_v_set != v_set;
        self.last_v_set = v_set;
        ProcessSetSpeedOutput { v_set, v_update }
    }
}
pub struct SpeedLimiterOnInput {
    pub prev_on_state: SpeedLimiterOn,
    pub activation_req: ActivationRequest,
    pub vacuum_brake_state: VacuumBrakeState,
    pub kickdown: KickdownState,
    pub speed: f64,
    pub v_set: f64,
}
pub struct SpeedLimiterOnOutput {
    pub on_state: SpeedLimiterOn,
    pub in_reg: bool,
}
pub struct SpeedLimiterOnState {
    last_hysterisis: Hysterisis,
}
impl grust::core::Component for SpeedLimiterOnState {
    type Input = SpeedLimiterOnInput;
    type Output = SpeedLimiterOnOutput;
    fn init() -> SpeedLimiterOnState {
        SpeedLimiterOnState {
            last_hysterisis: new_hysterisis(0.0f64),
        }
    }
    fn step(&mut self, input: SpeedLimiterOnInput) -> SpeedLimiterOnOutput {
        let prev_hysterisis = self.last_hysterisis;
        let (hysterisis, on_state) = match input.prev_on_state {
            SpeedLimiterOn::StandBy
                if activation_condition(
                    input.activation_req,
                    input.vacuum_brake_state,
                    input.v_set,
                ) =>
            {
                let on_state = SpeedLimiterOn::Actif;
                let hysterisis = new_hysterisis(0.0f64);
                (hysterisis, on_state)
            }
            SpeedLimiterOn::OverrideVoluntary
                if exit_override_condition(
                    input.activation_req,
                    input.kickdown,
                    input.v_set,
                    input.speed,
                ) =>
            {
                let on_state = SpeedLimiterOn::Actif;
                let hysterisis = new_hysterisis(0.0f64);
                (hysterisis, on_state)
            }
            SpeedLimiterOn::OverrideInvoluntary
                if exit_override_condition(
                    input.activation_req,
                    input.kickdown,
                    input.v_set,
                    input.speed,
                ) =>
            {
                let on_state = SpeedLimiterOn::Actif;
                let hysterisis = new_hysterisis(0.0f64);
                (hysterisis, on_state)
            }
            SpeedLimiterOn::OverrideVoluntary
                if involuntary_override_condition(
                    input.activation_req,
                    input.kickdown,
                    input.v_set,
                    input.speed,
                ) =>
            {
                let on_state = SpeedLimiterOn::OverrideInvoluntary;
                let hysterisis = prev_hysterisis;
                (hysterisis, on_state)
            }
            SpeedLimiterOn::Actif
                if voluntary_override_condition(input.activation_req, input.kickdown) =>
            {
                let on_state = SpeedLimiterOn::OverrideVoluntary;
                let hysterisis = prev_hysterisis;
                (hysterisis, on_state)
            }
            SpeedLimiterOn::Actif
                if standby_condition(
                    input.activation_req,
                    input.vacuum_brake_state,
                    input.v_set,
                ) =>
            {
                let on_state = SpeedLimiterOn::StandBy;
                let hysterisis = prev_hysterisis;
                (hysterisis, on_state)
            }
            SpeedLimiterOn::Actif => {
                let on_state = input.prev_on_state;
                let hysterisis = update_hysterisis(prev_hysterisis, input.speed, input.v_set);
                (hysterisis, on_state)
            }
            _ => {
                let hysterisis = prev_hysterisis;
                let on_state = input.prev_on_state;
                (hysterisis, on_state)
            }
        };
        let in_reg = in_regulation(hysterisis);
        self.last_hysterisis = hysterisis;
        SpeedLimiterOnOutput { on_state, in_reg }
    }
}
pub struct SpeedLimiterInput {
    pub activation_req: ActivationRequest,
    pub vacuum_brake_state: VacuumBrakeState,
    pub kickdown: KickdownState,
    pub vdc_disabled: VdcState,
    pub speed: f64,
    pub v_set: f64,
    pub clock: Option<f64>,
}
pub struct SpeedLimiterOutput {
    pub state: SpeedLimiter,
    pub on_state: SpeedLimiterOn,
    pub in_regulation: bool,
    pub state_update: bool,
}
pub struct SpeedLimiterState {
    last_in_regulation: bool,
    last_on_state: SpeedLimiterOn,
    last_state: SpeedLimiter,
    speed_limiter_on: SpeedLimiterOnState,
}
impl grust::core::Component for SpeedLimiterState {
    type Input = SpeedLimiterInput;
    type Output = SpeedLimiterOutput;
    fn init() -> SpeedLimiterState {
        SpeedLimiterState {
            last_in_regulation: true,
            last_on_state: SpeedLimiterOn::StandBy,
            last_state: SpeedLimiter::Off,
            speed_limiter_on: <SpeedLimiterOnState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: SpeedLimiterInput) -> SpeedLimiterOutput {
        let failure = false;
        let prev_state = self.last_state;
        let prev_on_state = self.last_on_state;
        let (in_regulation, on_state, state) = match (input.clock) {
            (Some(clock)) => {
                let (in_regulation, on_state, state) = match prev_state {
                    _ if off_condition(input.activation_req, input.vdc_disabled) => {
                        let state = SpeedLimiter::Off;
                        let on_state = prev_on_state;
                        let in_regulation = false;
                        (in_regulation, on_state, state)
                    }
                    SpeedLimiter::Off if on_condition(input.activation_req) => {
                        let (in_regulation, on_state, state) = match failure {
                            true => {
                                let state = SpeedLimiter::Fail;
                                let on_state = prev_on_state;
                                let in_regulation = false;
                                (in_regulation, on_state, state)
                            }
                            false => {
                                let state = SpeedLimiter::On;
                                let on_state = SpeedLimiterOn::StandBy;
                                let in_regulation = true;
                                (in_regulation, on_state, state)
                            }
                        };
                        (in_regulation, on_state, state)
                    }
                    SpeedLimiter::On if failure => {
                        let state = SpeedLimiter::Fail;
                        let on_state = prev_on_state;
                        let in_regulation = false;
                        (in_regulation, on_state, state)
                    }
                    SpeedLimiter::Fail if !(failure) => {
                        let state = SpeedLimiter::On;
                        let on_state = SpeedLimiterOn::StandBy;
                        let in_regulation = true;
                        (in_regulation, on_state, state)
                    }
                    SpeedLimiter::On => {
                        let state = prev_state;
                        let (on_state, in_regulation) = {
                            let SpeedLimiterOnOutput { on_state, in_reg } =
                                <SpeedLimiterOnState as grust::core::Component>::step(
                                    &mut self.speed_limiter_on,
                                    SpeedLimiterOnInput {
                                        prev_on_state: prev_on_state,
                                        activation_req: input.activation_req,
                                        vacuum_brake_state: input.vacuum_brake_state,
                                        kickdown: input.kickdown,
                                        speed: input.speed,
                                        v_set: input.v_set,
                                    },
                                );
                            (on_state, in_reg)
                        };
                        (in_regulation, on_state, state)
                    }
                    _ => {
                        let state = prev_state;
                        let on_state = prev_on_state;
                        let in_regulation = self.last_in_regulation;
                        (in_regulation, on_state, state)
                    }
                };
                (in_regulation, on_state, state)
            }
            (_) => {
                let (state, on_state, in_regulation) =
                    (self.last_state, self.last_on_state, self.last_in_regulation);
                (in_regulation, on_state, state)
            }
        };
        let state_update = (state != prev_state) || (on_state != prev_on_state);
        self.last_in_regulation = in_regulation;
        self.last_on_state = on_state;
        self.last_state = state;
        SpeedLimiterOutput {
            state,
            on_state,
            in_regulation,
            state_update,
        }
    }
}
pub mod runtime {
    use super::*;
    use grust::futures::{sink::SinkExt, stream::StreamExt};
    #[derive(Debug)]
    pub enum RuntimeInput {
        VacuumBrake(VacuumBrakeState, std::time::Instant),
        Activation(ActivationRequest, std::time::Instant),
        Kickdown(KickdownState, std::time::Instant),
        SetSpeed(f64, std::time::Instant),
        Vdc(VdcState, std::time::Instant),
        Speed(f64, std::time::Instant),
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
                (I::VacuumBrake(this, _), I::VacuumBrake(other, _)) => this.eq(other),
                (I::Activation(this, _), I::Activation(other, _)) => this.eq(other),
                (I::Kickdown(this, _), I::Kickdown(other, _)) => this.eq(other),
                (I::SetSpeed(this, _), I::SetSpeed(other, _)) => this.eq(other),
                (I::Vdc(this, _), I::Vdc(other, _)) => this.eq(other),
                (I::Speed(this, _), I::Speed(other, _)) => this.eq(other),
                (I::Timer(this, _), I::Timer(other, _)) => this.eq(other),
                _ => false,
            }
        }
    }
    impl RuntimeInput {
        pub fn get_instant(&self) -> std::time::Instant {
            match self {
                I::VacuumBrake(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Activation(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Kickdown(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::SetSpeed(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Vdc(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Speed(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Timer(_, _grust_reserved_instant) => *_grust_reserved_instant,
            }
        }
        pub fn order(v1: &Self, v2: &Self) -> std::cmp::Ordering {
            v1.get_instant().cmp(&v2.get_instant())
        }
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeOutput {
        InRegulation(bool, std::time::Instant),
        VSet(f64, std::time::Instant),
    }
    use RuntimeOutput as O;
    #[derive(Debug, Default)]
    pub struct RuntimeInit {
        pub vacuum_brake: VacuumBrakeState,
        pub activation: ActivationRequest,
        pub kickdown: KickdownState,
        pub set_speed: f64,
        pub vdc: VdcState,
        pub speed: f64,
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeTimer {
        PeriodClock,
    }
    use RuntimeTimer as T;
    impl grust::core::timer_stream::Timing for RuntimeTimer {
        fn get_duration(&self) -> std::time::Duration {
            match self {
                T::PeriodClock => std::time::Duration::from_millis(1000u64),
            }
        }
        fn do_reset(&self) -> bool {
            match self {
                T::PeriodClock => false,
            }
        }
    }
    pub struct Runtime {
        _grust_reserved_init_instant: std::time::Instant,
        speed_limiter: speed_limiter_service::SpeedLimiterService,
        output: grust::futures::channel::mpsc::Sender<O>,
        timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
    }
    impl Runtime {
        pub fn new(
            _grust_reserved_init_instant: std::time::Instant,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        ) -> Runtime {
            let speed_limiter = speed_limiter_service::SpeedLimiterService::init(
                _grust_reserved_init_instant,
                output.clone(),
                timer.clone(),
            );
            Runtime {
                _grust_reserved_init_instant,
                speed_limiter,
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
                vacuum_brake,
                activation,
                kickdown,
                set_speed,
                vdc,
                speed,
            } = init_vals;
            runtime
                .speed_limiter
                .handle_init(vacuum_brake, activation, kickdown, set_speed, vdc, speed)
                .await?;
            while let Some(input) = input.next().await {
                match input {
                    I::Speed(speed, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_speed(_grust_reserved_instant, speed)
                            .await?;
                    }
                    I::Timer(T::PeriodClock, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_period_clock(_grust_reserved_instant)
                            .await?;
                    }
                    I::VacuumBrake(vacuum_brake, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_vacuum_brake(_grust_reserved_instant, vacuum_brake)
                            .await?;
                    }
                    I::Activation(activation, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_activation(_grust_reserved_instant, activation)
                            .await?;
                    }
                    I::Kickdown(kickdown, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_kickdown(_grust_reserved_instant, kickdown)
                            .await?;
                    }
                    I::SetSpeed(set_speed, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_set_speed(_grust_reserved_instant, set_speed)
                            .await?;
                    }
                    I::Vdc(vdc, _grust_reserved_instant) => {
                        runtime
                            .speed_limiter
                            .handle_vdc(_grust_reserved_instant, vdc)
                            .await?;
                    }
                }
            }
            Ok(())
        }
    }
    pub mod speed_limiter_service {
        use super::*;
        use grust::futures::{sink::SinkExt, stream::StreamExt};
        mod ctx_ty {
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct InRegulation(bool, bool);
            impl InRegulation {
                pub fn set(&mut self, in_regulation: bool) {
                    self.1 = self.0 != in_regulation;
                    self.0 = in_regulation;
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
            pub struct VSetAux(f64, bool);
            impl VSetAux {
                pub fn set(&mut self, v_set_aux: f64) {
                    self.1 = self.0 != v_set_aux;
                    self.0 = v_set_aux;
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
            pub struct VSet(f64, bool);
            impl VSet {
                pub fn set(&mut self, v_set: f64) {
                    self.1 = self.0 != v_set;
                    self.0 = v_set;
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
            pub struct VacuumBrake(super::VacuumBrakeState, bool);
            impl VacuumBrake {
                pub fn set(&mut self, vacuum_brake: super::VacuumBrakeState) {
                    self.1 = self.0 != vacuum_brake;
                    self.0 = vacuum_brake;
                }
                pub fn get(&self) -> super::VacuumBrakeState {
                    self.0
                }
                pub fn take(&mut self) -> super::VacuumBrakeState {
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
            pub struct OnState(super::SpeedLimiterOn, bool);
            impl OnState {
                pub fn set(&mut self, on_state: super::SpeedLimiterOn) {
                    self.1 = self.0 != on_state;
                    self.0 = on_state;
                }
                pub fn get(&self) -> super::SpeedLimiterOn {
                    self.0
                }
                pub fn take(&mut self) -> super::SpeedLimiterOn {
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
            pub struct State(super::SpeedLimiter, bool);
            impl State {
                pub fn set(&mut self, state: super::SpeedLimiter) {
                    self.1 = self.0 != state;
                    self.0 = state;
                }
                pub fn get(&self) -> super::SpeedLimiter {
                    self.0
                }
                pub fn take(&mut self) -> super::SpeedLimiter {
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
            pub struct Vdc(super::VdcState, bool);
            impl Vdc {
                pub fn set(&mut self, vdc: super::VdcState) {
                    self.1 = self.0 != vdc;
                    self.0 = vdc;
                }
                pub fn get(&self) -> super::VdcState {
                    self.0
                }
                pub fn take(&mut self) -> super::VdcState {
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
            pub struct VUpdate(bool, bool);
            impl VUpdate {
                pub fn set(&mut self, v_update: bool) {
                    self.1 = self.0 != v_update;
                    self.0 = v_update;
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
            pub struct Kickdown(super::KickdownState, bool);
            impl Kickdown {
                pub fn set(&mut self, kickdown: super::KickdownState) {
                    self.1 = self.0 != kickdown;
                    self.0 = kickdown;
                }
                pub fn get(&self) -> super::KickdownState {
                    self.0
                }
                pub fn take(&mut self) -> super::KickdownState {
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
            pub struct StateUpdate(bool, bool);
            impl StateUpdate {
                pub fn set(&mut self, state_update: bool) {
                    self.1 = self.0 != state_update;
                    self.0 = state_update;
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
            pub struct Activation(super::ActivationRequest, bool);
            impl Activation {
                pub fn set(&mut self, activation: super::ActivationRequest) {
                    self.1 = self.0 != activation;
                    self.0 = activation;
                }
                pub fn get(&self) -> super::ActivationRequest {
                    self.0
                }
                pub fn take(&mut self) -> super::ActivationRequest {
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
            pub struct SetSpeed(f64, bool);
            impl SetSpeed {
                pub fn set(&mut self, set_speed: f64) {
                    self.1 = self.0 != set_speed;
                    self.0 = set_speed;
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
            pub struct InRegulationAux(bool, bool);
            impl InRegulationAux {
                pub fn set(&mut self, in_regulation_aux: bool) {
                    self.1 = self.0 != in_regulation_aux;
                    self.0 = in_regulation_aux;
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
            pub struct Speed(f64, bool);
            impl Speed {
                pub fn set(&mut self, speed: f64) {
                    self.1 = self.0 != speed;
                    self.0 = speed;
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
            pub in_regulation: ctx_ty::InRegulation,
            pub v_set_aux: ctx_ty::VSetAux,
            pub v_set: ctx_ty::VSet,
            pub vacuum_brake: ctx_ty::VacuumBrake,
            pub on_state: ctx_ty::OnState,
            pub state: ctx_ty::State,
            pub vdc: ctx_ty::Vdc,
            pub v_update: ctx_ty::VUpdate,
            pub kickdown: ctx_ty::Kickdown,
            pub state_update: ctx_ty::StateUpdate,
            pub activation: ctx_ty::Activation,
            pub set_speed: ctx_ty::SetSpeed,
            pub in_regulation_aux: ctx_ty::InRegulationAux,
            pub speed: ctx_ty::Speed,
        }
        impl Context {
            fn init() -> Context {
                Default::default()
            }
            fn reset(&mut self) {
                self.in_regulation.reset();
                self.v_set_aux.reset();
                self.v_set.reset();
                self.vacuum_brake.reset();
                self.on_state.reset();
                self.state.reset();
                self.vdc.reset();
                self.v_update.reset();
                self.kickdown.reset();
                self.state_update.reset();
                self.activation.reset();
                self.set_speed.reset();
                self.in_regulation_aux.reset();
                self.speed.reset();
            }
        }
        #[derive(Default)]
        pub struct SpeedLimiterServiceStore {
            vacuum_brake: Option<(VacuumBrakeState, std::time::Instant)>,
            activation: Option<(ActivationRequest, std::time::Instant)>,
            period_clock: Option<((), std::time::Instant)>,
            kickdown: Option<(KickdownState, std::time::Instant)>,
            set_speed: Option<(f64, std::time::Instant)>,
            vdc: Option<(VdcState, std::time::Instant)>,
            speed: Option<(f64, std::time::Instant)>,
        }
        impl SpeedLimiterServiceStore {
            pub fn not_empty(&self) -> bool {
                self.vacuum_brake.is_some()
                    || self.activation.is_some()
                    || self.period_clock.is_some()
                    || self.kickdown.is_some()
                    || self.set_speed.is_some()
                    || self.vdc.is_some()
                    || self.speed.is_some()
            }
        }
        pub struct SpeedLimiterService {
            _grust_reserved_init_instant: std::time::Instant,
            context: Context,
            delayed: bool,
            input_store: SpeedLimiterServiceStore,
            speed_limiter: SpeedLimiterState,
            process_set_speed: ProcessSetSpeedState,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        }
        impl SpeedLimiterService {
            pub fn init(
                _grust_reserved_init_instant: std::time::Instant,
                output: grust::futures::channel::mpsc::Sender<O>,
                timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
            ) -> SpeedLimiterService {
                let context = Context::init();
                let delayed = true;
                let input_store = Default::default();
                let speed_limiter = <SpeedLimiterState as grust::core::Component>::init();
                let process_set_speed = <ProcessSetSpeedState as grust::core::Component>::init();
                SpeedLimiterService {
                    _grust_reserved_init_instant,
                    context,
                    delayed,
                    input_store,
                    speed_limiter,
                    process_set_speed,
                    output,
                    timer,
                }
            }
            pub async fn handle_init(
                &mut self,
                vacuum_brake: VacuumBrakeState,
                activation: ActivationRequest,
                kickdown: KickdownState,
                set_speed: f64,
                vdc: VdcState,
                speed: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                let _grust_reserved_instant = self._grust_reserved_init_instant;
                let clock_ref = &mut None;
                self.context.speed.set(speed);
                self.context.vdc.set(vdc);
                self.context.set_speed.set(set_speed);
                let ProcessSetSpeedOutput {
                    v_set: v_set_aux,
                    v_update: v_update,
                } = <ProcessSetSpeedState as grust::core::Component>::step(
                    &mut self.process_set_speed,
                    ProcessSetSpeedInput {
                        set_speed: set_speed,
                    },
                );
                self.context.v_set_aux.set(v_set_aux);
                self.context.v_update.set(v_update);
                let v_set = self.context.v_set_aux.get();
                self.context.v_set.set(v_set);
                self.send_output(
                    O::VSet(v_set, _grust_reserved_instant),
                    _grust_reserved_instant,
                )
                .await?;
                self.context.kickdown.set(kickdown);
                self.send_timer(T::PeriodClock, _grust_reserved_instant)
                    .await?;
                *clock_ref = Some(
                    (_grust_reserved_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64,
                );
                self.context.activation.set(activation);
                self.context.vacuum_brake.set(vacuum_brake);
                let SpeedLimiterOutput {
                    state: state,
                    on_state: on_state,
                    in_regulation: in_regulation_aux,
                    state_update: state_update,
                } = <SpeedLimiterState as grust::core::Component>::step(
                    &mut self.speed_limiter,
                    SpeedLimiterInput {
                        activation_req: activation,
                        vacuum_brake_state: vacuum_brake,
                        kickdown: kickdown,
                        vdc_disabled: vdc,
                        speed: speed,
                        v_set: v_set,
                        clock: *clock_ref,
                    },
                );
                self.context.state.set(state);
                self.context.on_state.set(on_state);
                self.context.in_regulation_aux.set(in_regulation_aux);
                self.context.state_update.set(state_update);
                let in_regulation = self.context.in_regulation_aux.get();
                self.context.in_regulation.set(in_regulation);
                self.send_output(
                    O::InRegulation(in_regulation, _grust_reserved_instant),
                    _grust_reserved_instant,
                )
                .await?;
                Ok(())
            }
            pub async fn handle_vacuum_brake(
                &mut self,
                _vacuum_brake_instant: std::time::Instant,
                vacuum_brake: VacuumBrakeState,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_vacuum_brake_instant).await?;
                    self.context.reset();
                    self.context.vacuum_brake.set(vacuum_brake);
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: vacuum_brake,
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: self.context.vdc.get(),
                                speed: self.context.speed.get(),
                                v_set: self.context.v_set.get(),
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _vacuum_brake_instant),
                            _vacuum_brake_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .vacuum_brake
                        .replace((vacuum_brake, _vacuum_brake_instant));
                    assert ! (unique . is_none () , "flow `vacuum_brake` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_activation(
                &mut self,
                _activation_instant: std::time::Instant,
                activation: ActivationRequest,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_activation_instant).await?;
                    self.context.reset();
                    self.context.activation.set(activation);
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: activation,
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: self.context.vdc.get(),
                                speed: self.context.speed.get(),
                                v_set: self.context.v_set.get(),
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _activation_instant),
                            _activation_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .activation
                        .replace((activation, _activation_instant));
                    assert ! (unique . is_none () , "flow `activation` changes twice within one minimal delay of the service, consider reducing this delay");
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
                    if clock_ref.is_some()
                        || self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: self.context.vdc.get(),
                                speed: self.context.speed.get(),
                                v_set: self.context.v_set.get(),
                                clock: *clock_ref,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _period_clock_instant),
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
            pub async fn handle_kickdown(
                &mut self,
                _kickdown_instant: std::time::Instant,
                kickdown: KickdownState,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_kickdown_instant).await?;
                    self.context.reset();
                    self.context.kickdown.set(kickdown);
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: kickdown,
                                vdc_disabled: self.context.vdc.get(),
                                speed: self.context.speed.get(),
                                v_set: self.context.v_set.get(),
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _kickdown_instant),
                            _kickdown_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .kickdown
                        .replace((kickdown, _kickdown_instant));
                    assert ! (unique . is_none () , "flow `kickdown` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_set_speed(
                &mut self,
                _set_speed_instant: std::time::Instant,
                set_speed: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_set_speed_instant).await?;
                    self.context.reset();
                    self.context.set_speed.set(set_speed);
                    if self.context.set_speed.is_new() {
                        let ProcessSetSpeedOutput {
                            v_set: v_set_aux,
                            v_update: v_update,
                        } = <ProcessSetSpeedState as grust::core::Component>::step(
                            &mut self.process_set_speed,
                            ProcessSetSpeedInput {
                                set_speed: set_speed,
                            },
                        );
                        self.context.v_set_aux.set(v_set_aux);
                        self.context.v_update.set(v_update);
                    }
                    let v_set = self.context.v_set_aux.get();
                    self.context.v_set.set(v_set);
                    if self.context.v_set.is_new() {
                        self.send_output(O::VSet(v_set, _set_speed_instant), _set_speed_instant)
                            .await?;
                    }
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: self.context.vdc.get(),
                                speed: self.context.speed.get(),
                                v_set: v_set,
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _set_speed_instant),
                            _set_speed_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .set_speed
                        .replace((set_speed, _set_speed_instant));
                    assert ! (unique . is_none () , "flow `set_speed` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_vdc(
                &mut self,
                _vdc_instant: std::time::Instant,
                vdc: VdcState,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_vdc_instant).await?;
                    self.context.reset();
                    self.context.vdc.set(vdc);
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: vdc,
                                speed: self.context.speed.get(),
                                v_set: self.context.v_set.get(),
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _vdc_instant),
                            _vdc_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self.input_store.vdc.replace((vdc, _vdc_instant));
                    assert ! (unique . is_none () , "flow `vdc` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_speed(
                &mut self,
                _speed_instant: std::time::Instant,
                speed: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_speed_instant).await?;
                    self.context.reset();
                    self.context.speed.set(speed);
                    if self.context.activation.is_new()
                        || self.context.vacuum_brake.is_new()
                        || self.context.kickdown.is_new()
                        || self.context.vdc.is_new()
                        || self.context.speed.is_new()
                        || self.context.v_set.is_new()
                    {
                        let SpeedLimiterOutput {
                            state: state,
                            on_state: on_state,
                            in_regulation: in_regulation_aux,
                            state_update: state_update,
                        } = <SpeedLimiterState as grust::core::Component>::step(
                            &mut self.speed_limiter,
                            SpeedLimiterInput {
                                activation_req: self.context.activation.get(),
                                vacuum_brake_state: self.context.vacuum_brake.get(),
                                kickdown: self.context.kickdown.get(),
                                vdc_disabled: self.context.vdc.get(),
                                speed: speed,
                                v_set: self.context.v_set.get(),
                                clock: None,
                            },
                        );
                        self.context.state.set(state);
                        self.context.on_state.set(on_state);
                        self.context.in_regulation_aux.set(in_regulation_aux);
                        self.context.state_update.set(state_update);
                    }
                    let in_regulation = self.context.in_regulation_aux.get();
                    self.context.in_regulation.set(in_regulation);
                    if self.context.in_regulation.is_new() {
                        self.send_output(
                            O::InRegulation(in_regulation, _speed_instant),
                            _speed_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self.input_store.speed.replace((speed, _speed_instant));
                    assert ! (unique . is_none () , "flow `speed` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            #[inline]
            pub async fn reset_time_constraints(
                &mut self,
                instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                Ok(())
            }
            #[inline]
            pub async fn send_output(
                &mut self,
                output: O,
                instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
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
    const TIMER_CHANNEL_SIZE: usize = 1usize + 2;
    let (timers_sink, timers_stream) = grust::futures::channel::mpsc::channel(TIMER_CHANNEL_SIZE);
    let timers_stream = timers_stream.map(
        |(timer, instant): (runtime::RuntimeTimer, std::time::Instant)| {
            let deadline = instant + grust::core::timer_stream::Timing::get_duration(&timer);
            runtime::RuntimeInput::Timer(timer, deadline)
        },
    );
    const OUTPUT_CHANNEL_SIZE: usize = 2usize;
    let (output_sink, output_stream) = grust::futures::channel::mpsc::channel(OUTPUT_CHANNEL_SIZE);
    const PRIO_STREAM_SIZE: usize = 100usize;
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
