pub mod runtime {
    use super::*;
    use grust::futures::{sink::SinkExt, stream::StreamExt};
    #[derive(Debug)]
    pub enum RuntimeInput {
        InputS(i64, std::time::Instant),
        InputE(i64, std::time::Instant),
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
                (I::InputS(this, _), I::InputS(other, _)) => this.eq(other),
                (I::InputE(this, _), I::InputE(other, _)) => this.eq(other),
                (I::Timer(this, _), I::Timer(other, _)) => this.eq(other),
                _ => false,
            }
        }
    }
    impl RuntimeInput {
        pub fn get_instant(&self) -> std::time::Instant {
            match self {
                I::InputS(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::InputE(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Timer(_, _grust_reserved_instant) => *_grust_reserved_instant,
            }
        }
        pub fn order(v1: &Self, v2: &Self) -> std::cmp::Ordering {
            v1.get_instant().cmp(&v2.get_instant())
        }
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeOutput {
        Scanned(i64, std::time::Instant),
        Sampled(i64, std::time::Instant),
    }
    use RuntimeOutput as O;
    #[derive(Debug, Default)]
    pub struct RuntimeInit {
        pub input_s: i64,
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeTimer {
        PeriodClock,
        DelayTest,
        TimeoutTest,
    }
    use RuntimeTimer as T;
    impl grust::core::timer_stream::Timing for RuntimeTimer {
        fn get_duration(&self) -> std::time::Duration {
            match self {
                T::PeriodClock => std::time::Duration::from_millis(100u64),
                T::DelayTest => std::time::Duration::from_millis(10u64),
                T::TimeoutTest => std::time::Duration::from_millis(2000u64),
            }
        }
        fn do_reset(&self) -> bool {
            match self {
                T::PeriodClock => false,
                T::DelayTest => true,
                T::TimeoutTest => true,
            }
        }
    }
    pub struct Runtime {
        _grust_reserved_init_instant: std::time::Instant,
        test: test_service::TestService,
        output: grust::futures::channel::mpsc::Sender<O>,
        timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
    }
    impl Runtime {
        pub fn new(
            _grust_reserved_init_instant: std::time::Instant,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        ) -> Runtime {
            let test = test_service::TestService::init(
                _grust_reserved_init_instant,
                output.clone(),
                timer.clone(),
            );
            Runtime {
                _grust_reserved_init_instant,
                test,
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
            let RuntimeInit { input_s } = init_vals;
            runtime.test.handle_init(input_s).await?;
            while let Some(input) = input.next().await {
                match input {
                    I::InputE(input_e, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_input_e(_grust_reserved_instant, input_e)
                            .await?;
                    }
                    I::Timer(T::PeriodClock, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_period_clock(_grust_reserved_instant)
                            .await?;
                    }
                    I::Timer(T::DelayTest, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_delay_test(_grust_reserved_instant)
                            .await?;
                    }
                    I::InputS(input_s, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_input_s(_grust_reserved_instant, input_s)
                            .await?;
                    }
                    I::Timer(T::TimeoutTest, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_timeout_test(_grust_reserved_instant)
                            .await?;
                    }
                }
            }
            Ok(())
        }
    }
    pub mod test_service {
        use super::*;
        use grust::futures::{sink::SinkExt, stream::StreamExt};
        mod ctx_ty {
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct Scanned(i64, bool);
            impl Scanned {
                pub fn set(&mut self, scanned: i64) {
                    self.1 = self.0 != scanned;
                    self.0 = scanned;
                }
                pub fn get(&self) -> i64 {
                    self.0
                }
                pub fn take(&mut self) -> i64 {
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
            pub struct Sampled(Option<i64>, bool);
            impl Sampled {
                pub fn set(&mut self, sampled: Option<i64>) {
                    self.1 = self.0 != sampled;
                    self.0 = sampled;
                }
                pub fn get(&self) -> Option<i64> {
                    self.0
                }
                pub fn take(&mut self) -> Option<i64> {
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
            pub struct InputE(Option<i64>, bool);
            impl InputE {
                pub fn set(&mut self, input_e: Option<i64>) {
                    self.1 = self.0 != input_e;
                    self.0 = input_e;
                }
                pub fn get(&self) -> Option<i64> {
                    self.0
                }
                pub fn take(&mut self) -> Option<i64> {
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
            pub struct InputS(i64, bool);
            impl InputS {
                pub fn set(&mut self, input_s: i64) {
                    self.1 = self.0 != input_s;
                    self.0 = input_s;
                }
                pub fn get(&self) -> i64 {
                    self.0
                }
                pub fn take(&mut self) -> i64 {
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
            pub scanned: ctx_ty::Scanned,
            pub sampled: ctx_ty::Sampled,
            pub input_e: ctx_ty::InputE,
            pub input_s: ctx_ty::InputS,
        }
        impl Context {
            fn init() -> Context {
                Default::default()
            }
            fn reset(&mut self) {
                self.scanned.reset();
                self.sampled.reset();
                self.input_e.reset();
                self.input_s.reset();
            }
        }
        #[derive(Default)]
        pub struct TestServiceStore {
            period_clock: Option<((), std::time::Instant)>,
            input_s: Option<(i64, std::time::Instant)>,
            input_e: Option<(i64, std::time::Instant)>,
        }
        impl TestServiceStore {
            pub fn not_empty(&self) -> bool {
                self.period_clock.is_some() || self.input_s.is_some() || self.input_e.is_some()
            }
        }
        pub struct TestService {
            _grust_reserved_init_instant: std::time::Instant,
            context: Context,
            delayed: bool,
            input_store: TestServiceStore,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        }
        impl TestService {
            pub fn init(
                _grust_reserved_init_instant: std::time::Instant,
                output: grust::futures::channel::mpsc::Sender<O>,
                timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
            ) -> TestService {
                let context = Context::init();
                let delayed = true;
                let input_store = Default::default();
                TestService {
                    _grust_reserved_init_instant,
                    context,
                    delayed,
                    input_store,
                    output,
                    timer,
                }
            }
            pub async fn handle_init(
                &mut self,
                input_s: i64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                let _grust_reserved_instant = self._grust_reserved_init_instant;
                self.reset_service_timeout(_grust_reserved_instant).await?;
                let clock_ref = &mut None;
                let sampled_ref = &mut None;
                self.context.input_s.set(input_s);
                self.send_timer(T::PeriodClock, _grust_reserved_instant)
                    .await?;
                *clock_ref = Some(
                    (_grust_reserved_instant
                        .duration_since(self._grust_reserved_init_instant)
                        .as_millis()) as f64,
                );
                if clock_ref.is_some() {
                    *sampled_ref = self.context.input_e.take();
                }
                if let Some(sampled) = *sampled_ref {
                    self.send_output(
                        O::Sampled(sampled, _grust_reserved_instant),
                        _grust_reserved_instant,
                    )
                    .await?;
                }
                if clock_ref.is_some() {
                    self.context.scanned.set(input_s);
                }
                self.send_output(
                    O::Scanned(self.context.scanned.get(), _grust_reserved_instant),
                    _grust_reserved_instant,
                )
                .await?;
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
                    let sampled_ref = &mut None;
                    self.send_timer(T::PeriodClock, _period_clock_instant)
                        .await?;
                    *clock_ref = Some(
                        (_period_clock_instant
                            .duration_since(self._grust_reserved_init_instant)
                            .as_millis()) as f64,
                    );
                    if clock_ref.is_some() {
                        *sampled_ref = self.context.input_e.take();
                    }
                    if let Some(sampled) = *sampled_ref {
                        self.send_output(
                            O::Sampled(sampled, _period_clock_instant),
                            _period_clock_instant,
                        )
                        .await?;
                    }
                    if clock_ref.is_some() {
                        self.context.scanned.set(self.context.input_s.get());
                    }
                    if self.context.scanned.is_new() {
                        self.send_output(
                            O::Scanned(self.context.scanned.get(), _period_clock_instant),
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
            pub async fn handle_delay_test(
                &mut self,
                _grust_reserved_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.context.reset();
                if self.input_store.not_empty() {
                    self.reset_time_constraints(_grust_reserved_instant).await?;
                    let period_clock_ref = &mut None;
                    let input_e_ref = &mut None;
                    let clock_ref = &mut None;
                    let sampled_ref = &mut None;
                    let _input_e_input_store = self.input_store.input_e.take();
                    *input_e_ref = _input_e_input_store.map(|(x, _)| x);
                    let _input_s_input_store = self.input_store.input_s.take();
                    if let Some((input_s, _)) = _input_s_input_store {
                        self.context.input_s.set(input_s);
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
                    if input_e_ref.is_some() {
                        self.context.input_e.set(*input_e_ref);
                    }
                    if clock_ref.is_some() {
                        *sampled_ref = self.context.input_e.take();
                    }
                    if let Some(sampled) = *sampled_ref {
                        self.send_output(
                            O::Sampled(sampled, _grust_reserved_instant),
                            _grust_reserved_instant,
                        )
                        .await?;
                    }
                    if clock_ref.is_some() {
                        self.context.scanned.set(self.context.input_s.get());
                    }
                    if self.context.scanned.is_new() {
                        self.send_output(
                            O::Scanned(self.context.scanned.get(), _grust_reserved_instant),
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
                    .send((T::DelayTest, _grust_reserved_instant))
                    .await?;
                self.delayed = false;
                Ok(())
            }
            pub async fn handle_input_s(
                &mut self,
                _input_s_instant: std::time::Instant,
                input_s: i64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_input_s_instant).await?;
                    self.context.reset();
                    self.context.input_s.set(input_s);
                    if self.context.scanned.is_new() {
                        self.send_output(
                            O::Scanned(self.context.scanned.get(), _input_s_instant),
                            _input_s_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .input_s
                        .replace((input_s, _input_s_instant));
                    assert ! (unique . is_none () , "flow `input_s` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_timeout_test(
                &mut self,
                _timeout_test_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.reset_time_constraints(_timeout_test_instant).await?;
                self.context.reset();
                self.send_output(
                    O::Scanned(self.context.scanned.get(), _timeout_test_instant),
                    _timeout_test_instant,
                )
                .await?;
                Ok(())
            }
            #[inline]
            pub async fn reset_service_timeout(
                &mut self,
                _timeout_test_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.timer
                    .send((T::TimeoutTest, _timeout_test_instant))
                    .await?;
                Ok(())
            }
            pub async fn handle_input_e(
                &mut self,
                _input_e_instant: std::time::Instant,
                input_e: i64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_input_e_instant).await?;
                    self.context.reset();
                    let input_e_ref = &mut None;
                    *input_e_ref = Some(input_e);
                    if input_e_ref.is_some() {
                        self.context.input_e.set(*input_e_ref);
                    }
                } else {
                    let unique = self
                        .input_store
                        .input_e
                        .replace((input_e, _input_e_instant));
                    assert ! (unique . is_none () , "flow `input_e` changes twice within one minimal delay of the service, consider reducing this delay");
                }
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
    const OUTPUT_CHANNEL_SIZE: usize = 2usize;
    let (output_sink, output_stream) = grust::futures::channel::mpsc::channel(OUTPUT_CHANNEL_SIZE);
    const PRIO_STREAM_SIZE: usize = 3usize;
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
