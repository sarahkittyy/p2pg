use crate::GameState;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align2, Context, Pos2},
    EguiContexts,
};

fn center_pos(ctx: &mut Context) -> Pos2 {
    (ctx.screen_rect().size() / 2.0).to_pos2()
}

pub fn main_menu(mut ctxs: EguiContexts, mut next_state: ResMut<NextState<GameState>>) {
    let ctx = ctxs.ctx_mut();
    egui::Window::new("Menu")
        .pivot(Align2::CENTER_CENTER)
        .default_pos(center_pos(ctx))
        .show(ctx, |ui| {
            if ui.button("Quick Play").clicked() {
                next_state.set(GameState::Connecting);
            }
        });
}

pub fn connecting(mut ctxs: EguiContexts) {
    let ctx = ctxs.ctx_mut();
    egui::Window::new("Waiting for players...")
        .title_bar(false)
        .pivot(Align2::CENTER_CENTER)
        .default_pos(center_pos(ctx))
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.heading("Waiting for opponent...");
        });
}
