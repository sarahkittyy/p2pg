use crate::GameState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

pub fn main_menu(mut ctxs: EguiContexts, mut next_state: ResMut<NextState<GameState>>) {
    egui::Window::new("Menu").show(ctxs.ctx_mut(), |ui| {
        if ui.button("Quick Play").clicked() {
            next_state.set(GameState::Connecting);
        }
    });
}
