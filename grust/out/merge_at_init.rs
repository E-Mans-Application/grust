pub mod runtime {
    use super::*;
    use grust::futures::{sink::SinkExt, stream::StreamExt};
    #[derive(Debug)]
    pub enum RuntimeInput {
        Measure(f64, std::time::Instant),
        Stabilize(f64, std::time::Instant),
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
                (I::Measure(this, _), I::Measure(other, _)) => this.eq(other),
                (I::Stabilize(this, _), I::Stabilize(other, _)) => this.eq(other),
                (I::Timer(this, _), I::Timer(other, _)) => this.eq(other),
                _ => false,
            }
        }
    }
    impl RuntimeInput {
        pub fn get_instant(&self) -> std::time::Instant {
            match self {
                I::Measure(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Stabilize(_, _grust_reserved_instant) => *_grust_reserved_instant,
                I::Timer(_, _grust_reserved_instant) => *_grust_reserved_instant,
            }
        }
        pub fn order(v1: &Self, v2: &Self) -> std::cmp::Ordering {
            v1.get_instant().cmp(&v2.get_instant())
        }
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeOutput {
        ComputeEv(f64, std::time::Instant),
    }
    use RuntimeOutput as O;
    #[derive(Debug, Default)]
    pub struct RuntimeInit {
        pub measure: f64,
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeTimer {
        DelayKalmanTask,
        TimeoutKalmanTask,
    }
    use RuntimeTimer as T;
    impl grust::core::timer_stream::Timing for RuntimeTimer {
        fn get_duration(&self) -> std::time::Duration {
            match self {
                T::DelayKalmanTask => std::time::Duration::from_millis(10u64),
                T::TimeoutKalmanTask => std::time::Duration::from_millis(3000u64),
            }
        }
        fn do_reset(&self) -> bool {
            match self {
                T::DelayKalmanTask => true,
                T::TimeoutKalmanTask => true,
            }
        }
    }
    pub struct Runtime {
        _grust_reserved_init_instant: std::time::Instant,
        kalman_task: kalman_task_service::KalmanTaskService,
        output: grust::futures::channel::mpsc::Sender<O>,
        timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
    }
    impl Runtime {
        pub fn new(
            _grust_reserved_init_instant: std::time::Instant,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        ) -> Runtime {
            let kalman_task = kalman_task_service::KalmanTaskService::init(
                _grust_reserved_init_instant,
                output.clone(),
                timer.clone(),
            );
            Runtime {
                _grust_reserved_init_instant,
                kalman_task,
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
            let RuntimeInit { measure } = init_vals;
            runtime.kalman_task.handle_init(measure).await?;
            while let Some(input) = input.next().await {
                match input {
                    I::Timer(T::DelayKalmanTask, _grust_reserved_instant) => {
                        runtime
                            .kalman_task
                            .handle_delay_kalman_task(_grust_reserved_instant)
                            .await?;
                    }
                    I::Timer(T::TimeoutKalmanTask, _grust_reserved_instant) => {
                        runtime
                            .kalman_task
                            .handle_timeout_kalman_task(_grust_reserved_instant)
                            .await?;
                    }
                    I::Measure(measure, _grust_reserved_instant) => {
                        runtime
                            .kalman_task
                            .handle_measure(_grust_reserved_instant, measure)
                            .await?;
                    }
                    I::Stabilize(stabilize, _grust_reserved_instant) => {
                        runtime
                            .kalman_task
                            .handle_stabilize(_grust_reserved_instant, stabilize)
                            .await?;
                    }
                }
            }
            Ok(())
        }
    }
    pub mod kalman_task_service {
        use super::*;
        use grust::futures::{sink::SinkExt, stream::StreamExt};
        mod ctx_ty {
            #[derive(Clone, Copy, PartialEq, Default, Debug)]
            pub struct MeasureEvOld(f64, bool);
            impl MeasureEvOld {
                pub fn set(&mut self, measure_ev_old: f64) {
                    self.1 = self.0 != measure_ev_old;
                    self.0 = measure_ev_old;
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
            pub struct Measure(f64, bool);
            impl Measure {
                pub fn set(&mut self, measure: f64) {
                    self.1 = self.0 != measure;
                    self.0 = measure;
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
            pub measure_ev_old: ctx_ty::MeasureEvOld,
            pub measure: ctx_ty::Measure,
        }
        impl Context {
            fn init() -> Context {
                Default::default()
            }
            fn reset(&mut self) {
                self.measure_ev_old.reset();
                self.measure.reset();
            }
        }
        #[derive(Default)]
        pub struct KalmanTaskServiceStore {
            measure: Option<(f64, std::time::Instant)>,
            stabilize: Option<(f64, std::time::Instant)>,
        }
        impl KalmanTaskServiceStore {
            pub fn not_empty(&self) -> bool {
                self.measure.is_some() || self.stabilize.is_some()
            }
        }
        pub struct KalmanTaskService {
            _grust_reserved_init_instant: std::time::Instant,
            context: Context,
            delayed: bool,
            input_store: KalmanTaskServiceStore,
            output: grust::futures::channel::mpsc::Sender<O>,
            timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
        }
        impl KalmanTaskService {
            pub fn init(
                _grust_reserved_init_instant: std::time::Instant,
                output: grust::futures::channel::mpsc::Sender<O>,
                timer: grust::futures::channel::mpsc::Sender<(T, std::time::Instant)>,
            ) -> KalmanTaskService {
                let context = Context::init();
                let delayed = true;
                let input_store = Default::default();
                KalmanTaskService {
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
                measure: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                let _grust_reserved_instant = self._grust_reserved_init_instant;
                self.reset_service_timeout(_grust_reserved_instant).await?;
                let compute_ev_ref = &mut None;
                self.context.measure.set(measure);
                self.context.measure_ev_old.set(measure);
                if let Some(compute_ev) = *compute_ev_ref {
                    self.send_output(
                        O::ComputeEv(compute_ev, _grust_reserved_instant),
                        _grust_reserved_instant,
                    )
                    .await?;
                }
                Ok(())
            }
            pub async fn handle_delay_kalman_task(
                &mut self,
                _grust_reserved_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.context.reset();
                if self.input_store.not_empty() {
                    self.reset_time_constraints(_grust_reserved_instant).await?;
                    let stabilize_ref = &mut None;
                    let measure_ev_ref = &mut None;
                    let compute_ev_ref = &mut None;
                    let _stabilize_input_store = self.input_store.stabilize.take();
                    *stabilize_ref = _stabilize_input_store.map(|(x, _)| x);
                    let _measure_input_store = self.input_store.measure.take();
                    if let Some((measure, _)) = _measure_input_store {
                        self.context.measure.set(measure);
                    }
                    if self.context.measure_ev_old.get() != self.context.measure.get() {
                        self.context.measure_ev_old.set(self.context.measure.get());
                        *measure_ev_ref = Some(self.context.measure.get());
                    }
                    if measure_ev_ref.is_some() {
                        *compute_ev_ref = *measure_ev_ref;
                    } else {
                        if stabilize_ref.is_some() {
                            *compute_ev_ref = *stabilize_ref;
                        }
                    }
                    if let Some(compute_ev) = *compute_ev_ref {
                        self.send_output(
                            O::ComputeEv(compute_ev, _grust_reserved_instant),
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
                    .send((T::DelayKalmanTask, _grust_reserved_instant))
                    .await?;
                self.delayed = false;
                Ok(())
            }
            pub async fn handle_timeout_kalman_task(
                &mut self,
                _timeout_kalman_task_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.reset_time_constraints(_timeout_kalman_task_instant)
                    .await?;
                self.context.reset();
                Ok(())
            }
            #[inline]
            pub async fn reset_service_timeout(
                &mut self,
                _timeout_kalman_task_instant: std::time::Instant,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                self.timer
                    .send((T::TimeoutKalmanTask, _timeout_kalman_task_instant))
                    .await?;
                Ok(())
            }
            pub async fn handle_measure(
                &mut self,
                _measure_instant: std::time::Instant,
                measure: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_measure_instant).await?;
                    self.context.reset();
                    let measure_ev_ref = &mut None;
                    let compute_ev_ref = &mut None;
                    self.context.measure.set(measure);
                    if self.context.measure_ev_old.get() != measure {
                        self.context.measure_ev_old.set(measure);
                        *measure_ev_ref = Some(measure);
                    }
                    if measure_ev_ref.is_some() {
                        *compute_ev_ref = *measure_ev_ref;
                    }
                    if let Some(compute_ev) = *compute_ev_ref {
                        self.send_output(
                            O::ComputeEv(compute_ev, _measure_instant),
                            _measure_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .measure
                        .replace((measure, _measure_instant));
                    assert ! (unique . is_none () , "flow `measure` changes twice within one minimal delay of the service, consider reducing this delay");
                }
                Ok(())
            }
            pub async fn handle_stabilize(
                &mut self,
                _stabilize_instant: std::time::Instant,
                stabilize: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_stabilize_instant).await?;
                    self.context.reset();
                    let stabilize_ref = &mut None;
                    let compute_ev_ref = &mut None;
                    *stabilize_ref = Some(stabilize);
                    if stabilize_ref.is_some() {
                        *compute_ev_ref = *stabilize_ref;
                    }
                    if let Some(compute_ev) = *compute_ev_ref {
                        self.send_output(
                            O::ComputeEv(compute_ev, _stabilize_instant),
                            _stabilize_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .stabilize
                        .replace((stabilize, _stabilize_instant));
                    assert ! (unique . is_none () , "flow `stabilize` changes twice within one minimal delay of the service, consider reducing this delay");
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
    const TIMER_CHANNEL_SIZE: usize = 2usize + 2;
    const TIMER_STREAM_SIZE: usize = 2usize + 2;
    let (timers_sink, timers_stream) = grust::futures::channel::mpsc::channel(TIMER_CHANNEL_SIZE);
    let timers_stream =
        grust::core::timer_stream::timer_stream::<_, _, TIMER_STREAM_SIZE>(timers_stream)
            .map(|(timer, deadline)| runtime::RuntimeInput::Timer(timer, deadline));
    const OUTPUT_CHANNEL_SIZE: usize = 1usize;
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
