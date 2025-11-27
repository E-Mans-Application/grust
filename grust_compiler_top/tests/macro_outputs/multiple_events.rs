pub struct MultipleEventsInput {
    pub a: Option<i64>,
    pub b: Option<i64>,
    pub v: i64,
}
pub struct MultipleEventsOutput {
    pub c: i64,
    pub d: i64,
}
pub struct MultipleEventsState {
    last_a_bis: i64,
    last_d: i64,
    last_z: i64,
}
impl grust::core::Component for MultipleEventsState {
    type Input = MultipleEventsInput;
    type Output = MultipleEventsOutput;
    fn init() -> MultipleEventsState {
        MultipleEventsState {
            last_a_bis: 0i64,
            last_d: 0i64,
            last_z: 0i64,
        }
    }
    fn step(&mut self, input: MultipleEventsInput) -> MultipleEventsOutput {
        let (a_bis, z) = match (input.a, input.b) {
            (Some(a), Some(b)) if input.v > 50i64 => {
                let a_bis = a;
                let z = self.last_a_bis;
                (a_bis, z)
            }
            (Some(a), _) => {
                let a_bis = a;
                let z = 2i64;
                (a_bis, z)
            }
            (_, Some(b)) => {
                let a_bis = self.last_a_bis;
                let z = if input.v > 50i64 { 3i64 } else { 4i64 };
                (a_bis, z)
            }
            (_, _) => (self.last_a_bis, self.last_z),
        };
        let c = z;
        let d = match (input.a, input.b) {
            (Some(a), Some(b)) => {
                let d = (10i64 * a) + b;
                d
            }
            (_, _) => self.last_d,
        };
        self.last_a_bis = a_bis;
        self.last_d = d;
        self.last_z = z;
        MultipleEventsOutput { c, d }
    }
}
