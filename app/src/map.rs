use std::{io::Cursor, path::Path, sync::Arc};

use crate::{
    collision::{Hitbox, RigidBodyBundle},
    component::Tilemap,
};
use anyhow::anyhow;
use bevy::{
    asset::{AssetLoader, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{mesh::Indices, primitives::Aabb, render_resource::PrimitiveTopology},
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use tiled;

pub struct TiledPlugin;
impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TiledMap>()
            .add_asset_loader(TiledLoader)
            .add_systems(Update, tilemap_initializer);
    }
}

#[derive(Clone, TypeUuid, TypePath, Debug)]
#[uuid = "1c3fd034-5b2b-43b5-b878-45849790549e"]
pub struct TiledMap(pub tiled::Map);

#[derive(Bundle)]
struct TilemapBundle {
    tilemap: Tilemap,
    aabb: Aabb,
    spatial: SpatialBundle,
}

#[derive(Component)]
struct TilemapLoader {
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

fn tilemap_initializer(
    mut commands: Commands,
    q_loader: Query<(Entity, &TilemapLoader, Option<&Transform>)>,
    asset_server: Res<AssetServer>,
    maps: Res<Assets<TiledMap>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, loader, transform) in &q_loader {
        // wait for the tile map to be loaded...
        let tiledmap_handle: Handle<TiledMap> = asset_server.load(&loader.path);
        let Some(TiledMap(map)) = maps.get(&tiledmap_handle) else { continue; };

        // fetch the tileset image
        let tileset = map.tilesets().first().expect("Tileset not found...");
        let image = tileset
            .image
            .as_ref()
            .expect("Tileset has no associated image.");
        let tileset_handle = asset_server.load(image.source.clone());

        // tileset image material
        let material = ColorMaterial {
            texture: Some(tileset_handle.clone()),
            color: Color::WHITE,
        };
        let material_handle = materials.add(material);

        // parent tilemap entity
        let tilemap_entity = commands
            .spawn(TilemapBundle {
                tilemap: Tilemap,
                aabb: tilemap_aabb(map),
                spatial: SpatialBundle::from_transform(transform.cloned().unwrap_or_default()),
            })
            .id();

        // process each map layer
        for layer in map.layers() {
            let layer_type = layer.user_type.as_deref();
            match layer.layer_type() {
                tiled::LayerType::Tiles(layer) => {
                    let mesh = layer_to_mesh(map, &layer, tileset.as_ref());
                    let mesh_handle = meshes.add(mesh);

                    let layer_mesh_entity = commands
                        .spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(mesh_handle.clone()),
                            material: material_handle.clone(),
                            ..default()
                        })
                        .id();
                    commands.entity(tilemap_entity).add_child(layer_mesh_entity);
                }
                tiled::LayerType::Objects(layer) => {
                    if layer_type.is_some_and(|v| v == "collision") {
                        let hitboxes = layer_to_collision(&map, &layer);
                        hitboxes.into_iter().for_each(|(hitbox, transform)| {
                            let body = commands
                                .spawn(RigidBodyBundle::new(hitbox))
                                .insert(transform)
                                .id();
                            commands.entity(tilemap_entity).add_child(body);
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

    // clockwise uvs
    let uvs = [
        [0., 0.],
        [0., tile_uv_size.y],
        [tile_uv_size.x, tile_uv_size.y],
        [tile_uv_size.x, 0.],
    ] // a single uv quad at 0, 0
    .map(|[uvx, uvy]| [uvx + tile_uv0.x, uvy + tile_uv0.y]); // translated to uv0
    uvs
}

fn layer_to_mesh(map: &tiled::Map, layer: &tiled::TileLayer, tileset: &tiled::Tileset) -> Mesh {
    //NOTE: tiled renders right-down, but bevy is right-up (y is flipped)
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let quad: [[f32; 3]; 4] = [[0., 1., 0.], [0., 0., 0.], [1., 0., 0.], [1., 1., 0.]];

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
            let bevy_y = height - y - 1;
            //										 v since our y = 0 is tiled's y = 49
            let Some(tile) = layer.get_tile(x as i32, bevy_y as i32) else { continue; };
            let (xf, yf) = (x as f32, y as f32);

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
    mesh
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

struct TiledLoader;
impl AssetLoader for TiledLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let mut loader = tiled::Loader::with_cache_and_reader(
                tiled::DefaultResourceCache::new(),
                BytesResourceReader::new(bytes),
            );

            let map = loader
                .load_tmx_map(load_context.path())
                .map_err(|e| anyhow!("Could not load tmx map: {e}"))?;

            let loaded_map = LoadedAsset::new(TiledMap(map));

            load_context.set_default_asset(loaded_map);

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["tmx"]
    }
}
