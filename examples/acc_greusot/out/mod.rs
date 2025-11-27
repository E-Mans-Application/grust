use creusot_contracts::{ensures, logic, open, prelude, requires, DeepModel};
#[requires(0 < sv_v @ && sv_v @ <= 50)]
#[requires(0 < fv_v @ && fv_v @ < sv_v @ && sv_v @ - fv_v @ <= 10)]
#[ensures(0 < result @ && result @ < 150)]
#[ensures(result @ == logical :: safety_distance(sv_v @ , fv_v @))]
pub fn safety_distance(sv_v: i64, fv_v: i64) -> i64 {
    let sv_d_stop = (sv_v * (0i64 + 1i64)) + ((sv_v * sv_v) / (2i64 * 6i64));
    let fv_d_stop = (fv_v * fv_v) / (2i64 * 6i64);
    sv_d_stop - fv_d_stop
}
#[requires((0 < d_grace @ && d_grace @ < 150) && (v @ < 0 && - v @ <= 10))]
#[requires(d_grace @ > (v @ * v @) / (2 * 6))]
#[ensures(0 <= result @ && result @ <= 6)]
#[ensures(result @ == logical :: compute_braking(d_grace @ , v @))]
pub fn compute_braking(d_grace: i64, v: i64) -> i64 {
    (v * v) / (2i64 * d_grace)
}
pub struct AccInput {
    pub c: bool,
    pub d: i64,
    pub v: i64,
    pub s: i64,
}
pub struct AccOutput {
    pub b: i64,
}
pub struct AccState {}
impl grust::core::Component for AccState {
    type Input = AccInput;
    type Output = AccOutput;
    fn init() -> AccState {
        AccState {}
    }
    #[requires(input.d @ < 150)]
    #[requires(input.c == > (0 < input.s @ && input.s @ <= 50) &&
    (0 < input.s @ + input.v @ && input.v @ < 0 && - input.v @ <= 10))]
    #[requires(input.c == > input.d @ - logical ::
    safety_distance(input.s @ , input.s @ + input.v @) >
    (input.v @ * input.v @) / (2 * 6))]
    #[ensures(0 <= result.b @ && result.b @ <= 6)]
    fn step(&mut self, input: AccInput) -> AccOutput {
        let (d_safe, b, fv_v) = match input.c {
            true => {
                let fv_v = input.s + input.v;
                let d_safe = safety_distance(input.s, fv_v);
                let b = compute_braking(input.d - d_safe, input.v);
                (d_safe, b, fv_v)
            }
            false => {
                let b = 0i64;
                let (fv_v, d_safe) = (0i64, 0i64);
                (d_safe, b, fv_v)
            }
        };
        AccOutput { b }
    }
}
mod logical {
    use super::*;
    use creusot_contracts::{logic, open, Int};
    #[open]
    #[logic]
    pub fn safety_distance(sv_v: Int, fv_v: Int) -> Int {
        let sv_d_stop = (sv_v * (0 + 1)) + ((sv_v * sv_v) / (2 * 6));
        let fv_d_stop = (fv_v * fv_v) / (2 * 6);
        sv_d_stop - fv_d_stop
    }
    #[open]
    #[logic]
    pub fn compute_braking(d_grace: Int, v: Int) -> Int {
        (v * v) / (2 * d_grace)
    }
}
