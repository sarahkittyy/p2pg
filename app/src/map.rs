use std::{io::Cursor, path::Path, sync::Arc, time::Duration};

use crate::{
    animation::{AnimationBundle, AnimationIndices},
    collision::{Hitbox, RigidBodyBundle},
    component::Spawnpoint,
    MAP_FG_Z,
};
use anyhow::anyhow;
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{mesh::Indices, primitives::Aabb, render_resource::PrimitiveTopology},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    utils::BoxedFuture,
};
use tiled;

pub struct TiledPlugin;
impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Spawnpoint>()
            .register_asset_loader(TiledLoader)
            .init_asset::<TiledMap>()
            .add_systems(Update, tilemap_initializer);
    }
}

#[derive(Clone, TypeUuid, TypePath, Debug, Asset)]
#[uuid = "1c3fd034-5b2b-43b5-b878-45849790549e"]
pub struct TiledMap(pub tiled::Map);

#[derive(Bundle)]
struct TilemapBundle {
    tilemap: Tilemap,
    aabb: Aabb,
    spatial: SpatialBundle,
}

#[derive(Component)]
pub struct TilemapLoader {
    path: String,
}

#[derive(Bundle)]
pub struct TilemapLoaderBundle {
    loader: TilemapLoader,
}

impl TilemapLoaderBundle {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            loader: TilemapLoader { path: path.into() },
        }
    }
}

#[derive(Component)]
pub struct Tilemap;

#[derive(Bundle, Clone)]
struct AnimatedTileBundle {
    sprite: TextureAtlasSprite,
    animation: AnimationBundle,
    transform: Transform,
}

fn decompose_layer(
    map: &tiled::Map,
    layer: &tiled::TileLayer,
    tileset: &tiled::Tileset,
) -> (Mesh, Vec<AnimatedTileBundle>) {
    //NOTE: tiled renders right-down, but bevy is right-up (y is flipped)
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    let mut animated_tiles = vec![];

    // ccw vertices
    let quad: [[f32; 3]; 4] = [[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]];

    let mut positions: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    let mut indices: Vec<u32> = vec![];

    let image = tileset
        .image
        .as_ref()
        .expect("Tileset does not have an image");
    let image_size = Vec2::new(image.width as f32, image.height as f32);
    let tile_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);
    let tileset_tile_size = Vec2::new(tileset.tile_width as f32, tileset.tile_height as f32);

    let width = layer.width().unwrap();
    let height = layer.height().unwrap();

    // generate the mesh data for each tile
    for x in 0..width {
        for y in 0..height {
            let Some(tile) = layer.get_tile(x as i32, y as i32) else {
                continue;
            };
            let y = height - y - 1;
            let (xf, yf) = (x as f32, y as f32);

            if let Some(anim_tile) = tile.get_tile() {
                if let Some(frames) = &anim_tile.animation {
                    let tf = Transform::from_xyz(
                        (xf + 0.5) * tileset_tile_size.x,
                        (yf + 0.5) * tileset_tile_size.y,
                        0.,
                    );
                    let mut indices = AnimationIndices::from_frames(frames);
                    //TODO: diagonal flipping
                    indices.flip_x = tile.flip_h;
                    indices.flip_y = tile.flip_v;
                    // animated tile
                    animated_tiles.push(AnimatedTileBundle {
                        sprite: TextureAtlasSprite::new(0),
                        animation: AnimationBundle::new(
                            indices,
                            Timer::new(
                                Duration::from_millis(
                                    frames.iter().fold(0u64, |ms, f| ms + f.duration as u64)
                                        / frames.len() as u64,
                                ),
                                TimerMode::Repeating,
                            ),
                        ),
                        transform: tf,
                    });
                    continue;
                }
            }

            let [a, b, c, d] =
                quad.map(|[xp, yp, zp]| [(xp + xf) * tile_size.x, (yp + yf) * tile_size.y, zp]);
            let vc = positions.len() as u32;
            positions.extend([a, b, c, d]);
            normals.extend(vec![[0., 0., 1.]; 4]);
            indices.extend([0, 1, 2, 2, 3, 0].map(|i| i + vc));
            uvs.extend(tile_to_uvs(tile, image_size, tileset_tile_size));
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    (mesh, animated_tiles)
}

fn tilemap_initializer(
    mut commands: Commands,
    q_loader: Query<(Entity, &TilemapLoader, Option<&Transform>)>,
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TiledMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut atlases: ResMut<Assets<TextureAtlas>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, loader, transform) in &q_loader {
        // wait for the tile map to be loaded...
        let tiledmap_handle: Handle<TiledMap> = asset_server.load(&loader.path);
        let Some(TiledMap(map)) = maps.get(&tiledmap_handle) else {
            continue;
        };

        // fetch the tileset image
        let tileset = map.tilesets().first().expect("Tileset not found...");
        let image = tileset
            .image
            .as_ref()
            .expect("Tileset has no associated image.");
        let tileset_handle = asset_server.load(image.source.clone());
        let tileset_rows = image.height / tileset.tile_height as i32;

        // tileset image material
        let material = ColorMaterial {
            texture: Some(tileset_handle.clone()),
            color: Color::WHITE,
        };
        let material_handle = materials.add(material);

        let map_size = map_size(&map);
        let tileset_tile_size = UVec2::new(tileset.tile_width, tileset.tile_height).as_vec2();

        let map_tf = transform.cloned().unwrap_or_default();

        // parent tilemap entity
        commands.spawn(TilemapBundle {
            tilemap: Tilemap,
            aabb: tilemap_aabb(&map),
            spatial: SpatialBundle::from_transform(map_tf),
        });

        // process each map layer
        for (layer_i, layer) in map.layers().enumerate() {
            let layer_type = layer.user_type.as_deref();
            let layer_z_offset = layer_i as f32 * 0.1;
            match layer.layer_type() {
                tiled::LayerType::Tiles(layer) => {
                    let (mesh, animated) = decompose_layer(&map, &layer, tileset.as_ref());
                    let mesh_handle = meshes.add(mesh);

                    let z = layer_z_offset
                        + if layer_type.is_some_and(|s| s == "foreground") {
                            MAP_FG_Z
                        } else {
                            0.
                        };

                    // layer mesh
                    let layer_tf = Transform::from_xyz(0., 0., z).mul_transform(map_tf);
                    commands.spawn(MaterialMesh2dBundle {
                        mesh: Mesh2dHandle(mesh_handle.clone()),
                        material: material_handle.clone(),
                        transform: layer_tf,
                        ..default()
                    });
                    // animated tile entity
                    for tile in animated {
                        let tile_tf = tile.transform.mul_transform(layer_tf);
                        commands
                            .spawn(tile)
                            .insert(atlases.add(TextureAtlas::from_grid(
                                tileset_handle.clone(),
                                tileset_tile_size,
                                tileset.columns as usize,
                                tileset_rows as usize,
                                None,
                                None,
                            )))
                            .insert(SpatialBundle::from_transform(tile_tf));
                    }
                }
                tiled::LayerType::Objects(layer) => {
                    if layer_type.is_some_and(|v| v == "collision") {
                        let hitboxes = layer_to_collision(&map, &layer);
                        hitboxes.into_iter().for_each(|(hitbox, transform)| {
                            commands
                                .spawn(RigidBodyBundle::new(hitbox))
                                .insert(transform.mul_transform(map_tf));
                        });
                    } else if layer_type.is_some_and(|v| v == "spawnpoints") {
                        layer
                            .objects()
                            .filter_map(|object| match object.shape {
                                tiled::ObjectShape::Point(x, y) => {
                                    Some(Vec2::new(x, map_size.y - y))
                                }
                                _ => None,
                            })
                            .for_each(|spawnpoint| {
                                commands
                                    .spawn(Spawnpoint)
                                    .insert(TransformBundle::from_transform(map_tf.mul_transform(
                                        Transform::from_translation(spawnpoint.extend(0.)),
                                    )));
                            });
                    }
                }
                _ => (),
            }
        }

        // despawn the loader
        commands.entity(entity).despawn_recursive();
    }
}

fn layer_to_collision(map: &tiled::Map, layer: &tiled::ObjectLayer) -> Vec<(Hitbox, Transform)> {
    let mut hitboxes = vec![];

    let map_size = map_size(map);

    for object in layer.objects() {
        let mut pos = Vec2::new(object.x, object.y); // top left point in right-down coordinates
        pos.y = map_size.y - pos.y - 1.; // top left point in right-up coordinates
        match object.shape {
            tiled::ObjectShape::Rect { width, height } => {
                let size = Vec2::new(width, height);
                let center = Vec2::new(pos.x + size.x / 2., pos.y - size.y / 2.);
                hitboxes.push((
                    Hitbox::Rect {
                        offset: Vec2::ZERO,
                        half_size: size / 2.,
                    },
                    Transform::from_translation(center.extend(0.)),
                ));
            }
            tiled::ObjectShape::Ellipse { width, height } => {
                if width != height {
                    warn!("tilemap contains non-circular collision box");
                }
                let radius = width / 2.;
                let center = Vec2::new(pos.x + radius, pos.y - radius);
                hitboxes.push((
                    Hitbox::Circle {
                        offset: Vec2::ZERO,
                        radius,
                    },
                    Transform::from_translation(center.extend(0.)),
                ));
            }
            _ => (),
        }
    }

    hitboxes
}

fn tilemap_aabb(map: &tiled::Map) -> Aabb {
    let halfsize = map_size(map) / 2.;
    Aabb {
        center: halfsize.extend(0.).into(),
        half_extents: halfsize.extend(0.).into(),
    }
}

/// size of a tiled map, in pixels
fn map_size(map: &tiled::Map) -> Vec2 {
    UVec2 {
        x: map.tile_width * map.width,
        y: map.tile_height * map.height,
    }
    .as_vec2()
}

fn tile_to_uvs(tile: tiled::LayerTile, image_size: Vec2, tile_size: Vec2) -> [[f32; 2]; 4] {
    let id = tile.id() as u32;

    // columns in the tileset
    let columns = (image_size.x / tile_size.x).round() as u32;
    // xy position of the tile in the tileset
    let tileset_pos = UVec2::new(id % columns, id / columns).as_vec2();
    // size of a tile, normalized from 0 to 1
    let mut tile_uv_size = tile_size / image_size;
    // top-left uv coordinate
    let mut tile_uv0 = tileset_pos * tile_uv_size;

    // tiny offset to prevent imprecision artifacts
    let epsilon = Vec2::splat(0.0001);
    tile_uv0 += epsilon;
    tile_uv_size -= epsilon;

    // ccw uvs
    let [a, b, c, d] = [
        [0., tile_uv_size.y],
        [tile_uv_size.x, tile_uv_size.y],
        [tile_uv_size.x, 0.],
        [0., 0.],
    ] // a single uv quad at 0, 0
    .map(|[uvx, uvy]| [uvx + tile_uv0.x, uvy + tile_uv0.y]); // translated to uv0

    let [a, b, c, d] = if tile.flip_d {
        [c, b, a, d]
    } else {
        [a, b, c, d]
    };

    let [a, b, c, d] = if tile.flip_v {
        [d, c, b, a]
    } else {
        [a, b, c, d]
    };

    let [a, b, c, d] = if tile.flip_h {
        [b, a, d, c]
    } else {
        [a, b, c, d]
    };

    [a, b, c, d]
}

struct BytesResourceReader {
    bytes: Arc<[u8]>,
}

impl BytesResourceReader {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: Arc::from(bytes),
        }
    }
}

impl tiled::ResourceReader for BytesResourceReader {
    type Resource = Cursor<Arc<[u8]>>;
    type Error = std::io::Error;

    fn read_from(&mut self, _path: &Path) -> std::result::Result<Self::Resource, Self::Error> {
        // In this case, the path is ignored because the byte data is already provided.
        Ok(Cursor::new(self.bytes.clone()))
    }
}

#[derive(Default)]
struct TiledLoader;
impl AssetLoader for TiledLoader {
    type Asset = TiledMap;
    type Settings = ();
    type Error = anyhow::Error;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<Self::Asset>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut loader = tiled::Loader::with_cache_and_reader(
                tiled::DefaultResourceCache::new(),
                BytesResourceReader::new(&bytes),
            );

            let map = loader
                .load_tmx_map(load_context.path())
                .map_err(|e| anyhow!("Could not load tmx map: {e}"))?;

            Ok(TiledMap(map))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}
