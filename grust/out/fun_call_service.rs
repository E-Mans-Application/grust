pub mod runtime {
    use super::*;
    use grust::futures::{sink::SinkExt, stream::StreamExt};
    #[derive(Debug)]
    pub enum RuntimeInput {
        FloatSignal(f64, std::time::Instant),
    }
    use RuntimeInput as I;
    impl grust::core::priority_stream::Reset for RuntimeInput {
        fn do_reset(&self) -> bool {
            match self {
                _ => false,
            }
        }
    }
    impl PartialEq for RuntimeInput {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (I::FloatSignal(this, _), I::FloatSignal(other, _)) => this.eq(other),
                _ => false,
            }
        }
    }
    impl RuntimeInput {
        pub fn get_instant(&self) -> std::time::Instant {
            match self {
                I::FloatSignal(_, _grust_reserved_instant) => *_grust_reserved_instant,
            }
        }
        pub fn order(v1: &Self, v2: &Self) -> std::cmp::Ordering {
            v1.get_instant().cmp(&v2.get_instant())
        }
    }
    #[derive(Debug, PartialEq)]
    pub enum RuntimeOutput {
        IntSignal(i64, std::time::Instant),
    }
    use RuntimeOutput as O;
    #[derive(Debug, Default)]
    pub struct RuntimeInit {
        pub float_signal: f64,
    }
    pub struct Runtime {
        _grust_reserved_init_instant: std::time::Instant,
        test: test_service::TestService,
        output: grust::futures::channel::mpsc::Sender<O>,
    }
    impl Runtime {
        pub fn new(
            _grust_reserved_init_instant: std::time::Instant,
            output: grust::futures::channel::mpsc::Sender<O>,
        ) -> Runtime {
            let test =
                test_service::TestService::init(_grust_reserved_init_instant, output.clone());
            Runtime {
                _grust_reserved_init_instant,
                test,
                output,
            }
        }
        pub async fn run_loop(
            self,
            input: impl grust::futures::Stream<Item = I>,
            init_vals: RuntimeInit,
        ) -> Result<(), grust::futures::channel::mpsc::SendError> {
            grust::futures::pin_mut!(input);
            let mut runtime = self;
            let RuntimeInit { float_signal } = init_vals;
            runtime.test.handle_init(float_signal).await?;
            while let Some(input) = input.next().await {
                match input {
                    I::FloatSignal(float_signal, _grust_reserved_instant) => {
                        runtime
                            .test
                            .handle_float_signal(_grust_reserved_instant, float_signal)
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
            pub struct IntSignal(i64, bool);
            impl IntSignal {
                pub fn set(&mut self, int_signal: i64) {
                    self.1 = self.0 != int_signal;
                    self.0 = int_signal;
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
            pub struct FloatSignal(f64, bool);
            impl FloatSignal {
                pub fn set(&mut self, float_signal: f64) {
                    self.1 = self.0 != float_signal;
                    self.0 = float_signal;
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
            pub int_signal: ctx_ty::IntSignal,
            pub float_signal: ctx_ty::FloatSignal,
        }
        impl Context {
            fn init() -> Context {
                Default::default()
            }
            fn reset(&mut self) {
                self.int_signal.reset();
                self.float_signal.reset();
            }
        }
        #[derive(Default)]
        pub struct TestServiceStore {
            float_signal: Option<(f64, std::time::Instant)>,
        }
        impl TestServiceStore {
            pub fn not_empty(&self) -> bool {
                self.float_signal.is_some()
            }
        }
        pub struct TestService {
            _grust_reserved_init_instant: std::time::Instant,
            context: Context,
            delayed: bool,
            input_store: TestServiceStore,
            output: grust::futures::channel::mpsc::Sender<O>,
        }
        impl TestService {
            pub fn init(
                _grust_reserved_init_instant: std::time::Instant,
                output: grust::futures::channel::mpsc::Sender<O>,
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
                }
            }
            pub async fn handle_init(
                &mut self,
                float_signal: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                let _grust_reserved_instant = self._grust_reserved_init_instant;
                self.context.float_signal.set(float_signal);
                let int_signal = utils::floor(float_signal);
                self.context.int_signal.set(int_signal);
                self.send_output(
                    O::IntSignal(self.context.int_signal.get(), _grust_reserved_instant),
                    _grust_reserved_instant,
                )
                .await?;
                Ok(())
            }
            pub async fn handle_float_signal(
                &mut self,
                _float_signal_instant: std::time::Instant,
                float_signal: f64,
            ) -> Result<(), grust::futures::channel::mpsc::SendError> {
                if self.delayed {
                    self.reset_time_constraints(_float_signal_instant).await?;
                    self.context.reset();
                    self.context.float_signal.set(float_signal);
                    if self.context.float_signal.is_new() {
                        let int_signal = utils::floor(float_signal);
                        self.context.int_signal.set(int_signal);
                    }
                    if self.context.int_signal.is_new() {
                        self.send_output(
                            O::IntSignal(self.context.int_signal.get(), _float_signal_instant),
                            _float_signal_instant,
                        )
                        .await?;
                    }
                } else {
                    let unique = self
                        .input_store
                        .float_signal
                        .replace((float_signal, _float_signal_instant));
                    assert ! (unique . is_none () , "flow `float_signal` changes twice within one minimal delay of the service, consider reducing this delay");
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
        }
    }
}
use grust::futures::{Stream, StreamExt};
pub fn run(
    _grust_reserved_init_instant: std::time::Instant,
    input_stream: impl Stream<Item = runtime::RuntimeInput> + Send + 'static,
    init_signals: runtime::RuntimeInit,
) -> grust::futures::channel::mpsc::Receiver<runtime::RuntimeOutput> {
    const OUTPUT_CHANNEL_SIZE: usize = 1usize;
    let (output_sink, output_stream) = grust::futures::channel::mpsc::channel(OUTPUT_CHANNEL_SIZE);
    const PRIO_STREAM_SIZE: usize = 2usize;
    let prio_stream = grust::core::priority_stream::prio_stream::<_, _, PRIO_STREAM_SIZE>(
        input_stream,
        runtime::RuntimeInput::order,
    );
    let service = runtime::Runtime::new(_grust_reserved_init_instant, output_sink);
    grust::tokio::spawn(async move {
        let result = service.run_loop(prio_stream, init_signals).await;
        assert!(result.is_ok())
    });
    output_stream
}
