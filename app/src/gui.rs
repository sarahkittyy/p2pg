use std::collections::VecDeque;

use crate::{
    component::{Player, Points},
    p2p::LocalPlayer,
    GameState,
};
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

pub fn points_display(
    mut ctxs: EguiContexts,
    q_points: Query<(&Player, &Points)>,
    local_player: Res<LocalPlayer>,
) {
    let ctx = ctxs.ctx_mut();
    egui::Window::new("Points")
        .anchor(Align2::RIGHT_TOP, Vec2::ZERO)
        .resizable(false)
        .collapsible(true)
        .movable(false)
        .show(ctx, |ui| {
            for (player, points) in &q_points {
                ui.horizontal(|ui| {
                    if player.id == local_player.id {
                        ui.label("You: ");
                    } else {
                        ui.label("Them: ");
                    }
                    ui.monospace(format!("{}", points.0));
                });
            }
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
