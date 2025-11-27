grust_compiler_top::prelude! {}

#[test]
fn should_compile_para() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para.rs", service_para, mode = test]
        import event e0: int;
        export signal o1: int;

        component C1(e0: int?) -> (s2: int, e1: int?) {
            when {
                init => {
                    s2 = 0;
                }
                e0? if e0 > prev_s2 => {
                    s2 = e0;
                    e1 = emit e0 / (e0 - prev_s2);
                }
                e0? => {
                    s2 = e0;
                }
            }
            let prev_s2: int = last s2;
        }

        component C2(e1: int?) -> (s3: int, e3: int?) {
            when {
                init => {
                    s3 = 0;
                }
                e1? if e1 > 1 => {
                    s3 = e1;
                    e3 = emit (last s3);
                }
                e1? => {
                    s3 = e1;
                }
            }
        }

        component C3(s2: int) -> (e2: int?) {
            e2 = when { s2 > 1 => emit s2 };
        }

        component C4(e2: int?) -> (s4: int) {
            s4 = when { init => 0, e2? => e2 };
        }

        component C5(s4: int, s3: int, e3: int?) -> (o: int) {
            o = when {
                init => 0,
                e3? => e3,
                s4 > 0 => s4*2,
                s3 >= 0 => s3,
            };
        }

        service para_mess @ [10, 3000] {
            let (signal s2: int, event e1: int) = C1(e0);
            let (signal s3: int, event e3: int) = C2(e1);
            let event e2: int = C3(s2);
            let signal s4: int = C4(e2);
            o1 = C5(s4, s3, e3);
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

#[test]
fn should_compile_para_threads() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_threads.rs", component_para_threads]

        component test_threads_aux(i: int) -> (next_o: int) {
            let i1: int = (i - 54) * 2;
            let i2: int = (i + 54) * 2;
            let i3: int = 7 * i;
            let i12: int = i1 + i;
            let i23: int = i2 + i3;
            let i123: int = i12 + 2 * i3 + i23;
            init i = 0;
            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i123;
                },
                _ => {
                    next_o = i12;
                },
            }
        }

        component test_threads(i : int) -> (next_o: int) {
            let i1_1: int = (i - 54) * 2;
            let i1_2: int = (i + 54) * 2;
            let i1_3: int = test_threads_aux(i);

            let i2_1: int = test_threads_aux(i1_1 + i1_2 - i1_3);
            let i2_2: int = test_threads_aux(i1_2 - i1_2 + i1_3);

            init i = 0;

            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i2_1;
                },
                _ => {
                    next_o = i2_2;
                },
            }
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

#[test]
fn should_compile_para_rayon1() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_rayon1.rs", component_para_rayon1]

        component test_rayon1_aux(i: int) -> (next_o: int) {
            let i1: int = (i - 54) * 2;
            let i2: int = (i + 54) * 2;
            let i3: int = 7 * i;
            let i12: int = i1 + i2;
            let i23: int = i2 + i3;
            let i123: int = i12 + 2 * i3 + i23;
            init i = 0;
            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i123;
                },
                _ => {
                    next_o = i12;
                },
            }
        }

        component test_rayon1(i : int) -> (next_o: int) {
            let i1_1: int = (i - 54) * 2;
            let i1_2: int = (i + 54) * 2;
            let i1_3: int = test_rayon1_aux(i);

            let i2_1: int = test_rayon1_aux(i1_1 + i1_2 - i1_3);
            let i2_2: int = test_rayon1_aux(i1_2 - i1_2 + i1_3);

            init i = 0;

            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i2_1;
                },
                _ => {
                    next_o = i2_2;
                },
            }
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

#[test]
fn should_compile_para_rayon2() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_rayon2.rs", component_para_rayon2]

        component test_rayon2_aux(i: int) -> (next_o: int) {
            let i1: int = (i - 54) * 2;
            let i2: int = (i + 54) * 2;
            let i3: int = 7 * i;
            let i12: int = i1 + i2;
            let i23: int = i2 + i3;
            let i123: int = i12 + 2 * i3 + i23;
            init i = 0;
            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i123;
                },
                _ => {
                    next_o = i12;
                },
            }
        }

        component test_rayon2(i : int) -> (next_o: int) {
            let i1_1: int = (i - 54) * 2;
            let i1_2: int = (i + 54) * 2;
            let i1_3: int = test_rayon2_aux(i);

            let i2_1: int = test_rayon2_aux(i1_1 + i1_2 - i1_3);
            let i2_2: int = test_rayon2_aux(i1_2 - i1_2 + i1_3);

            init i = 0;

            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i2_1;
                },
                _ => {
                    next_o = i2_2;
                },
            }
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

#[test]
fn should_compile_rayon3() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_rayon3.rs", component_para_rayon3]

        component test_rayon3_aux(i: int) -> (next_o: int) {
            let i1: int = (i - 54) * 2;
            let i2: int = (i + 54) * 2;
            let i3: int = 7 * i;
            let i12: int = i1 + i2;
            let i23: int = i2 + i3;
            let i123: int = i12 + 2 * i3 + i23;
            init i = 0;
            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i123;
                },
                _ => {
                    next_o = i12;
                },
            }
        }

        component test_rayon3(i : int) -> (next_o: int) {
            let i1_1: int = (i - 54) * 2;
            let i1_2: int = (i + 54) * 2;
            let i1_3: int = test_rayon3_aux(i);

            let i2_1: int = test_rayon3_aux(i1_1 + i1_2 - i1_3);
            let i2_2: int = test_rayon3_aux(i1_2 - i1_2 + i1_3);

            init i = 0;

            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i2_1;
                },
                _ => {
                    next_o = i2_2;
                },
            }
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

#[test]
fn should_compile_para_mixed() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_mixed.rs", component_para_mixed]

        component test_mixed_aux(i: int) -> (next_o: int) {
            let i1: int = (i - 54) * 2;
            let i2: int = (i + 54) * 2;
            let i3: int = 7 * i;
            let i12: int = i1 + i2;
            let i23: int = i2 + i3;
            let i123: int = i12 + 2 * i3 + i23;
            init i = 0;
            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i123;
                },
                _ => {
                    next_o = i12;
                },
            }
        }

        component test_mixed(i : int) -> (next_o: int) {
            let i1_1: int = (i - 54) * 2;
            let i1_2: int = (i + 54) * 2;
            let i1_3: int = test_mixed_aux(i);

            let i2_1: int = test_mixed_aux(i1_1 + i1_2 - i1_3);
            let i2_2: int = test_mixed_aux(i1_2 - i1_2 + i1_3);

            init i = 0;

            match i {
                0 => {
                    next_o = 1 + last i;
                },
                7 => {
                    next_o = i2_1;
                },
                _ => {
                    next_o = i2_2;
                },
            }
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}

pub mod module {
    pub fn add_i64(n: i64, m: i64) -> i64 {
        n + m
    }
}

#[test]
fn should_compile_para_custom() {
    let top: ir0::Top = parse_quote! {
        #![dump = "tests/macro_outputs/para_custom.rs", component_para (2, 6, 20)]
        #[weight_percent = 5]
        use function module::add_i64(n: int, m: int) -> int;

        #[weight_percent = 10]
        function add(i: int, j: int) -> int {
            return add_i64(i, j);
        }

        #[weight_percent = 12]
        component next(i: int) -> (next_o: int) {
            init i = 1;
            next_o = add(i, last i);
        }

        component semi_fib(i: int) -> (o: int) {
            let next_o: int = next(i);
            o = last next_o;
            init next_o = 0;
        }

        component fib_call() -> (fib: int) {
            let next_o: int = next(fib);
            fib = semi_fib(next_o);
        }

        component fib() -> (fib: int) {
            init (fib, next_o) = (1, 0);
            let next_o: int = fib + last fib;
            fib = last next_o;
        }
    };
    let (ast, mut ctx) = top.init();
    let tokens = grust_compiler_top::into_token_stream_res(ast, &mut ctx).unwrap();
    if let Some(path) = ctx.conf.dump_code {
        grust_compiler_top::dump_code(&path, &tokens).unwrap();
    }
}
