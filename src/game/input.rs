//! What the player does, collected by the front end once per simulation step.
//! Keyboard, touch and gamepad all fill the same struct.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// The jump button is down.
    pub jump_held: bool,
    /// The jump button went down since the previous step (an edge – never lost between steps).
    pub jump_pressed: bool,
    /// Touch assist: a jump that started sideways grabs a ladder it passes, without "up" held
    /// (a thumb on a d-pad cannot hold sideways + up).
    pub auto_grab: bool,
}
