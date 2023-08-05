use std::collections::VecDeque;

use crate::GameState;
use bevy::prelude::*;
use bevy_egui::{
    egui::{self, Align2, Context, Pos2, Vec2},
    EguiContexts,
};

fn center_pos(ctx: &mut Context) -> Pos2 {
    (ctx.screen_rect().size() / 2.0).to_pos2()
}

pub fn fps_display(mut ctxs: EguiContexts, time: Res<Time>, mut history: Local<VecDeque<f32>>) {
    let ctx = ctxs.ctx_mut();
    let fps = (1. / time.delta_seconds()).round();
    history.push_front(fps);
    if history.len() > 40 {
        history.pop_back();
    }
    let avg: f32 = history.iter().sum::<f32>() / history.len() as f32;
    egui::Window::new("Fps Counter")
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .anchor(Align2::RIGHT_TOP, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.heading(avg.round().to_string());
        });
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
