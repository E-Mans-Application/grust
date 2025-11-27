use creusot_contracts::{ensures, logic, open, prelude, requires, DeepModel};
#[derive(prelude :: Clone, Copy, prelude :: PartialEq, DeepModel)]
pub enum Braking {
    UrgentBrake,
    SoftBrake,
    NoBrake,
}
# [requires (0 <= speed @ && speed @ < 50)]
# [ensures (result @ == logical :: compute_soft_braking_distance (speed @))]
pub fn compute_soft_braking_distance(speed: i64) -> i64 {
    (speed * speed) / 100i64
}
# [requires (0 <= speed @ && speed @ < 50)]
# [ensures (result != Braking :: NoBrake)]
# [ensures (result == logical :: brakes (distance @ , speed @))]
pub fn brakes(distance: i64, speed: i64) -> Braking {
    let braking_distance = compute_soft_braking_distance(speed);
    let response = if braking_distance < distance {
        Braking::SoftBrake
    } else {
        Braking::UrgentBrake
    };
    response
}
pub struct BrakingStateInput {
    pub pedest: Option<i64>,
    pub timeout_pedest: Option<()>,
    pub speed: i64,
}
pub struct BrakingStateOutput {
    pub state: Braking,
}
pub struct BrakingStateState {
    last_state: Braking,
}
impl grust::core::Component for BrakingStateState {
    type Input = BrakingStateInput;
    type Output = BrakingStateOutput;
    fn init() -> BrakingStateState {
        BrakingStateState {
            last_state: Braking::NoBrake,
        }
    }
    # [requires (0 <= input . speed @ && input . speed @ < 50)]
    # [ensures (forall < p : i64 > Some (p) == input . pedest == > result . state != Braking :: NoBrake)]
    fn step(&mut self, input: BrakingStateInput) -> BrakingStateOutput {
        let state = match (input.pedest, input.timeout_pedest) {
            (Some(d), _) => {
                let state = brakes(d, input.speed);
                state
            }
            (_, Some(_)) => {
                let state = Braking::NoBrake;
                state
            }
            (_, _) => self.last_state,
        };
        self.last_state = state;
        BrakingStateOutput { state }
    }
}
mod logical {
    use super::*;
    use creusot_contracts::{logic, open, Int};
    #[open]
    #[logic]
    pub fn compute_soft_braking_distance(speed: Int) -> Int {
        (speed * speed) / 100
    }
    #[open]
    #[logic]
    pub fn brakes(distance: Int, speed: Int) -> Braking {
        let braking_distance = compute_soft_braking_distance(speed);
        let response = if braking_distance < distance {
            Braking::SoftBrake
        } else {
            Braking::UrgentBrake
        };
        response
    }
}
