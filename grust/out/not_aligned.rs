pub struct AuxInput {
    pub i: i64,
}
pub struct AuxOutput {
    pub next_o: i64,
}
pub struct AuxState {
    last_i: i64,
}
impl grust::core::Component for AuxState {
    type Input = AuxInput;
    type Output = AuxOutput;
    fn init() -> AuxState {
        AuxState { last_i: 0i64 }
    }
    fn step(&mut self, input: AuxInput) -> AuxOutput {
        let ((i3, i1, i2), ()) = {
            let (i3, i1, i2) = {
                let (
                    reserved_grust_rayon_opt_var_0,
                    reserved_grust_rayon_opt_var_1,
                    reserved_grust_rayon_opt_var_2,
                ) = {
                    #[allow(unused_imports)]
                    use grust::rayon::prelude::*;
                    (0 .. 3usize) . into_par_iter () . map (| idx : usize | match idx {0usize => (Some ({7i64 * input . i}) , None , None) , 1usize => (None , Some ({(input . i - 54i64) * 2i64}) , None) , 2usize => (None , None , Some ({(input . i + 54i64) * 2i64})) , idx => unreachable ! ("fatal error in rayon branches, illegal index `{}`" , idx ,) ,}) . reduce (|| (None , None , None) , | (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_1 , reserved_grust_rayon_opt_var_2) , (reserved_grust_rayon_opt_var_0_rgt , reserved_grust_rayon_opt_var_1_rgt , reserved_grust_rayon_opt_var_2_rgt) | (match (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_0_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")} , match (reserved_grust_rayon_opt_var_1 , reserved_grust_rayon_opt_var_1_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")} , match (reserved_grust_rayon_opt_var_2 , reserved_grust_rayon_opt_var_2_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")}))
                };
                (
                    {
                        reserved_grust_rayon_opt_var_0 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                    {
                        reserved_grust_rayon_opt_var_1 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                    {
                        reserved_grust_rayon_opt_var_2 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                )
            };
            ((i3, i1, i2), ())
        };
        let ((i12, i23), ()) = {
            let (i12, i23) = {
                let (reserved_grust_rayon_opt_var_0, reserved_grust_rayon_opt_var_1) = {
                    #[allow(unused_imports)]
                    use grust::rayon::prelude::*;
                    (0 .. 2usize) . into_par_iter () . map (| idx : usize | match idx {0usize => (Some ({i1 + i2}) , None) , 1usize => (None , Some ({i2 + i3})) , idx => unreachable ! ("fatal error in rayon branches, illegal index `{}`" , idx ,) ,}) . reduce (|| (None , None) , | (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_1) , (reserved_grust_rayon_opt_var_0_rgt , reserved_grust_rayon_opt_var_1_rgt) | (match (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_0_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")} , match (reserved_grust_rayon_opt_var_1 , reserved_grust_rayon_opt_var_1_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")}))
                };
                (
                    {
                        reserved_grust_rayon_opt_var_0 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                    {
                        reserved_grust_rayon_opt_var_1 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                )
            };
            ((i12, i23), ())
        };
        let i123 = (i12 + (2i64 * i3)) + i23;
        let next_o = match input.i {
            0 => {
                let next_o = 1i64 + self.last_i;
                next_o
            }
            7 => {
                let next_o = i123;
                next_o
            }
            _ => {
                let next_o = i12;
                next_o
            }
        };
        self.last_i = input.i;
        AuxOutput { next_o }
    }
}
pub struct TestInput {
    pub i: i64,
}
pub struct TestOutput {
    pub next_o: i64,
}
pub struct TestState {
    last_i: i64,
    aux: AuxState,
    aux_1: AuxState,
    aux_2: AuxState,
}
impl grust::core::Component for TestState {
    type Input = TestInput;
    type Output = TestOutput;
    fn init() -> TestState {
        TestState {
            last_i: 0i64,
            aux: <AuxState as grust::core::Component>::init(),
            aux_1: <AuxState as grust::core::Component>::init(),
            aux_2: <AuxState as grust::core::Component>::init(),
        }
    }
    fn step(&mut self, input: TestInput) -> TestOutput {
        let ((i1_1, i1_2), i1_3) = std::thread::scope(|reserved_grust_thread_scope| {
            let reserved_grust_thread_kid_0 = reserved_grust_thread_scope.spawn(|| {
                let AuxOutput { next_o } = <AuxState as grust::core::Component>::step(
                    &mut self.aux,
                    AuxInput { i: input.i },
                );
                (next_o)
            });
            let (i1_1, i1_2) = {
                let (reserved_grust_rayon_opt_var_0, reserved_grust_rayon_opt_var_1) = {
                    #[allow(unused_imports)]
                    use grust::rayon::prelude::*;
                    (0 .. 2usize) . into_par_iter () . map (| idx : usize | match idx {0usize => (Some ({(input . i - 54i64) * 2i64}) , None) , 1usize => (None , Some ({(input . i + 54i64) * 2i64})) , idx => unreachable ! ("fatal error in rayon branches, illegal index `{}`" , idx ,) ,}) . reduce (|| (None , None) , | (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_1) , (reserved_grust_rayon_opt_var_0_rgt , reserved_grust_rayon_opt_var_1_rgt) | (match (reserved_grust_rayon_opt_var_0 , reserved_grust_rayon_opt_var_0_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")} , match (reserved_grust_rayon_opt_var_1 , reserved_grust_rayon_opt_var_1_rgt) {(None , None) => None , (Some (val) , None) | (None , Some (val)) => Some (val) , (Some (_) , Some (_)) => unreachable ! ("fatal error in rayon reduce operation, found two values")}))
                };
                (
                    {
                        reserved_grust_rayon_opt_var_0 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                    {
                        reserved_grust_rayon_opt_var_1 . expect ("unreachable: fatal error in final rayon unwrap, unexpected `None` value")
                    },
                )
            };
            let i1_3 = {
                reserved_grust_thread_kid_0
                    .join()
                    .expect("unexpected panic in sub-thread")
            };
            ((i1_1, i1_2), i1_3)
        });
        let ((x, i2_1), (x_1, i2_2)) = std::thread::scope(|reserved_grust_thread_scope| {
            let reserved_grust_thread_kid_0 = reserved_grust_thread_scope.spawn(|| {
                let x = (i1_1 + i1_2) - i1_3;
                let i2_1 = {
                    let AuxOutput { next_o } = <AuxState as grust::core::Component>::step(
                        &mut self.aux_1,
                        AuxInput { i: x },
                    );
                    (next_o)
                };
                (x, i2_1)
            });
            let reserved_grust_thread_kid_1 = reserved_grust_thread_scope.spawn(|| {
                let x_1 = (i1_2 - i1_2) + i1_3;
                let i2_2 = {
                    let AuxOutput { next_o } = <AuxState as grust::core::Component>::step(
                        &mut self.aux_2,
                        AuxInput { i: x_1 },
                    );
                    (next_o)
                };
                (x_1, i2_2)
            });
            let ((x, i2_1), (x_1, i2_2)) = (
                {
                    reserved_grust_thread_kid_0
                        .join()
                        .expect("unexpected panic in sub-thread")
                },
                {
                    reserved_grust_thread_kid_1
                        .join()
                        .expect("unexpected panic in sub-thread")
                },
            );
            ((x, i2_1), (x_1, i2_2))
        });
        let next_o = match input.i {
            0 => {
                let next_o = 1i64 + self.last_i;
                next_o
            }
            7 => {
                let next_o = i2_1;
                next_o
            }
            _ => {
                let next_o = i2_2;
                next_o
            }
        };
        self.last_i = input.i;
        TestOutput { next_o }
    }
}
