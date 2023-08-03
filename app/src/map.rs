use std::{io::Cursor, path::Path, sync::Arc};

use anyhow::anyhow;
use bevy::{
    asset::{AssetLoader, LoadedAsset},
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{mesh::Indices, render_resource::PrimitiveTopology},
};
use tiled;

#[derive(Clone, TypeUuid, TypePath, Debug)]
#[uuid = "1c3fd034-5b2b-43b5-b878-45849790549e"]
pub struct TiledMap(pub tiled::Map);

fn tile_to_uvs(id: u32, image_size: Vec2, tile_size: Vec2) -> [[f32; 2]; 4] {
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

    let uvs = [
        [0., 0.],
        [0., tile_uv_size.y],
        [tile_uv_size.x, tile_uv_size.y],
        [tile_uv_size.x, 0.],
    ] // a single uv quad at 0, 0
    .map(|[uvx, uvy]| [uvx + tile_uv0.x, uvy + tile_uv0.y]); // translated to uv0
    uvs
}

pub fn tilemap_to_mesh(map: &tiled::Map) -> Mesh {
    //NOTE: tiled renders right-down, but bevy is right-up (y is flipped)
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let quad: [[f32; 3]; 4] = [[0., 1., 0.], [0., 0., 0.], [1., 0., 0.], [1., 1., 0.]];

    let mut positions: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];
    let mut indices: Vec<u32> = vec![];

    let tileset = map
        .tilesets()
        .first()
        .expect("Expected at least one tileset in the map");
    let image = tileset
        .image
        .as_ref()
        .expect("Tileset does not have an image");
    let image_size = Vec2::new(image.width as f32, image.height as f32);
    let tile_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);
    let tileset_tile_size = Vec2::new(tileset.tile_width as f32, tileset.tile_height as f32);

    for layer in map.layers() {
        // we only care about tile layers right now
        let Some(layer) = layer.as_tile_layer() else { continue };

        let width = layer.width().unwrap();
        let height = layer.height().unwrap();

        // generate the mesh data for each tile
        for x in 0..width {
            for y in 0..height {
                //										 v since our y = 0 is tiled's y = 49
                let Some(tile) = layer.get_tile(x as i32, (height - y - 1) as i32) else { continue; };
                let (xf, yf) = (x as f32, y as f32);

                let [a, b, c, d] =
                    quad.map(|[xp, yp, zp]| [(xp + xf) * tile_size.x, (yp + yf) * tile_size.y, zp]);
                let vc = positions.len() as u32;
                positions.extend([a, b, c, d]);
                normals.extend(vec![[0., 0., 1.]; 4]);
                indices.extend([0, 1, 2, 2, 3, 0].map(|i| i + vc));
                uvs.extend(tile_to_uvs(tile.id() as u32, image_size, tileset_tile_size));
            }
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh
}

pub struct TiledPlugin;
impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TiledMap>().add_asset_loader(TiledLoader);
    }
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
