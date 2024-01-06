use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::{
        camera::{ScalingMode, Viewport},
        primitives::Aabb,
    },
};

use crate::{
    component::{FollowPlayer, MainCamera, MinimapCamera, Player, Tilemap},
    p2p::LocalPlayer,
};

/// spawn the primary game camera
pub fn spawn_primary(mut commands: Commands) {
    let ideal_aspect_ratio = 16f32 / 9.;
    let max_width = 16. * 25.;
    let max_height = max_width / ideal_aspect_ratio;

    // main camera
    commands
        .spawn(MainCamera)
        .insert(Camera2dBundle {
            projection: OrthographicProjection {
                // optimize view area on 16
                scaling_mode: ScalingMode::AutoMax {
                    max_height,
                    max_width,
                },
                ..default()
            },
            camera: Camera {
                order: 0,
                ..default()
            },
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
            },
            ..default()
        })
        .insert(FollowPlayer);
}

/// sets the camera to follow the local player, stopping at the tilemap boundaries
pub fn follow_player(
    local_player_id: Option<Res<LocalPlayer>>,
    q_player: Query<(&Player, &Transform)>,
    mut q_camera: Query<
        (&mut Transform, &OrthographicProjection),
        (With<FollowPlayer>, With<Camera>, Without<Player>),
    >,
    q_map: Query<(&Aabb, &Transform), (With<Tilemap>, Without<Camera>, Without<Player>)>,
) {
    let Some(id) = local_player_id else {
        return;
    };
    // tilemap aabb relative to itself
    let Ok((map_aabb, map_transform)) = q_map.get_single() else {
        return;
    };
    for (player, player_transform) in &q_player {
        if player.id != id.id {
            continue;
        }

        for (mut transform, proj) in &mut q_camera {
            let player_pos = player_transform.translation.truncate();
            let viewport_area = proj.area;

            let map_center = map_transform
                .transform_point(map_aabb.center.into())
                .truncate();
            let map_halfsize = (map_transform.scale * Vec3::from(map_aabb.half_extents)).truncate();

            // map boundary in world coordinates
            let map_min = map_center - map_halfsize;
            let map_max = map_center + map_halfsize;

            let camera_min = map_min + viewport_area.size() / 2.;
            let camera_max = map_max - viewport_area.size() / 2.;

            transform.translation.x = player_pos.x.clamp(camera_min.x, camera_max.x);
            transform.translation.y = player_pos.y.clamp(camera_min.y, camera_max.y);
        }
    }
}

/// spawns a tiny minimap in the top-left corner
pub fn spawn_minimap_camera(mut commands: Commands) {
    commands
        .spawn(MinimapCamera)
        .insert(Camera2dBundle {
            projection: OrthographicProjection {
                scaling_mode: ScalingMode::Fixed {
                    width: 400.,
                    height: 400.,
                },
                ..default()
            },
            camera: Camera {
                viewport: Some(Viewport {
                    physical_size: UVec2::splat(100),
                    ..default()
                }),
                order: 1,
                ..default()
            },
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::None,
            },
            ..default()
        })
        .insert(FollowPlayer);
}
