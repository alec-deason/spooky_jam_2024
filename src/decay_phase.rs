use bevy::{
    prelude::*,
};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash)]
enum PhasePhase {
    Running,
    ShuttingDown,
}
pub struct DecayPhasePlugin;

impl Plugin for DecayPhasePlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_state(PhasePhase::Running)
            .add_systems(OnEnter(crate::GameState::DecayPhase), |mut next_state: ResMut<NextState<PhasePhase>>| { next_state.set(PhasePhase::Running) })
            .add_systems(Update, (poop).run_if(in_state(PhasePhase::Running)).run_if(in_state(crate::GameState::DecayPhase)))
            ;
    }
}

fn poop() {
    println!("POOP");
}
