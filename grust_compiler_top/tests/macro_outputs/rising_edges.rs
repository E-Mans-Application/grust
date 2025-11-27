pub struct RisingEdgesInput {
    pub a: Option<i64>,
    pub b: Option<i64>,
    pub v: i64,
}
pub struct RisingEdgesOutput {
    pub c: i64,
    pub d: f64,
    pub x: Option<i64>,
}
pub struct RisingEdgesState {
    last_c: i64,
    last_d: f64,
    last_x_1: bool,
    last_x_2: bool,
    last_x_3: bool,
    last_z: i64,
}
impl grust::core::Component for RisingEdgesState {
    type Input = RisingEdgesInput;
    type Output = RisingEdgesOutput;
    fn init() -> RisingEdgesState {
        RisingEdgesState {
            last_c: 0i64,
            last_d: 0.0f64,
            last_x_1: false,
            last_x_2: false,
            last_x_3: false,
            last_z: 0i64,
        }
    }
    fn step(&mut self, input: RisingEdgesInput) -> RisingEdgesOutput {
        let x_1 = input.v > 50i64;
        let w = match () {
            () if x_1 && !(self.last_x_1) => {
                let w = Some(input.v + self.last_c);
                w
            }
            () => None,
        };
        let c = match (w, input.a) {
            (Some(w), _) => {
                let c = w;
                c
            }
            (_, Some(a)) => {
                let c = a;
                c
            }
            (_, _) => self.last_c,
        };
        let x_3 = input.v < 40i64;
        let x_2 = input.v > 50i64;
        let (z, y, x) = match (input.a, input.b) {
            (Some(a), Some(e)) if x_2 && !(self.last_x_2) => {
                let z = if input.v > 80i64 { e } else { a };
                let y = Some(());
                (z, y, None)
            }
            (Some(a), _) if (a != 0i64) && (x_3 && !(self.last_x_3)) => {
                let z = 2i64;
                let x = Some(2i64);
                (z, None, x)
            }
            (_, Some(e)) if e < 20i64 => {
                let z = self.last_z;
                let x = Some(2i64);
                (z, None, x)
            }
            (_, _) => (self.last_z, None, None),
        };
        let d = match (y) {
            (Some(_)) => {
                let d = 0.1f64;
                d
            }
            (_) => self.last_d,
        };
        self.last_c = c;
        self.last_d = d;
        self.last_x_1 = x_1;
        self.last_x_2 = x_2;
        self.last_x_3 = x_3;
        self.last_z = z;
        RisingEdgesOutput { c, d, x }
    }
}
