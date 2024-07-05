use bevy::{
    core::FrameCount,
    prelude::*,
    render::{render_asset::RenderAssetUsages, texture::ImageSampler},
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
};
use image::{Pixel, Rgba};
use noise::{Abs, Exponent, Fbm, MultiFractal, NoiseFn, Perlin};
use rayon::prelude::*;

#[derive(Component)]
struct Chunk {
    #[allow(dead_code)]
    pos: IVec2,
}
pub struct GenerationPlugin;

const SCALE: f32 = 8.0;
const SIZE: usize = 300_000_000;
const NOISE_SCALE: f64 = 2000.;
const SPAWN_CHUNKS: i32 = 16;
const CHUNK_SIZE: usize = 16;
const SIZE_BOUND: f64 = SIZE as f64 / NOISE_SCALE;
const X_EXTENT: f64 = SIZE_BOUND - -SIZE_BOUND;
const Y_EXTENT: f64 = SIZE_BOUND - -SIZE_BOUND;
const X_STEP: f64 = X_EXTENT / SIZE as f64;
const Y_STEP: f64 = Y_EXTENT / SIZE as f64;
const SEED: u32 = 1928877623;

impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, start_level);
        app.add_systems(Update, chunk_generated);
    }
}
async fn gen_chunk(chunk_pos: IVec2) -> Option<ChunkGenerationResult> {
    //let duration = Duration::from_secs_f32(rand::thread_rng().gen_range(0.05..5.0));
    //async_std::task::sleep(duration).await;

    let elevation =
        Abs::new(Exponent::new(Fbm::<Perlin>::new(SEED).set_frequency(0.8)).set_exponent(2.0));
    let moisture = Exponent::new(Fbm::<Perlin>::new(SEED)).set_exponent(0.5);
    let tint = Abs::new(Fbm::<Perlin>::new(SEED));
    let global_chunk_pos = chunk_pos * CHUNK_SIZE as i32;
    let mut texture = image::RgbaImage::new(CHUNK_SIZE as u32, CHUNK_SIZE as u32);
    texture
        .par_enumerate_pixels_mut()
        .for_each(|(x, y, pixel)| {
            let global_x = x as i32 + global_chunk_pos.x;
            let current_x = -SIZE_BOUND + X_STEP * global_x as f64;
            let global_y = y as i32 + global_chunk_pos.y;
            let current_y = -SIZE_BOUND + Y_STEP * global_y as f64;
            let e = elevation.get([current_x, current_y]) as f32;
            let m = moisture.get([current_x, current_y]) as f32;
            let tint = tint.get([global_x as f64, global_y as f64]) as f32;
            let mut color = biome(e, m).color();
            color.blend(&image::Rgba([
                (tint * 255.0) as u8,
                (tint * 255.0) as u8,
                (tint * 255.0) as u8,
                64,
            ]));
            *pixel = color
        });
    image::imageops::flip_vertical_in_place(&mut texture);
    Some(ChunkGenerationResult::new(texture))
}
enum Biome {
    Ocean,
    Beach,
    Scorched,
    Tundra,
    TemperateDesert,
    Shrubland,
    Grassland,
    TemperateDeciduousForest,
    TemperateRainForest,
    SubtropicalDesert,
    TropicalSeasonalForest,
    TropicalRainForest,
    Taiga,
    Snow,
}

fn biome(e: f32, m: f32) -> Biome {
    if e < 0.1 {
        return Biome::Ocean;
    }
    if e < 0.12 {
        return Biome::Beach;
    }

    if e > 0.8 {
        if m < 0.1 {
            return Biome::Scorched;
        }
        if m < 0.5 {
            return Biome::Tundra;
        }
        return Biome::Snow;
    }

    if e > 0.6 {
        if m < 0.33 {
            return Biome::TemperateDesert;
        }
        if m < 0.66 {
            return Biome::Shrubland;
        }
        return Biome::Taiga;
    }

    if e > 0.3 {
        if m < 0.16 {
            return Biome::TemperateDesert;
        }
        if m < 0.50 {
            return Biome::Grassland;
        }
        if m < 0.83 {
            return Biome::TemperateDeciduousForest;
        }
        return Biome::TemperateRainForest;
    }

    if m < 0.16 {
        return Biome::SubtropicalDesert;
    }
    if m < 0.33 {
        return Biome::Grassland;
    }
    if m < 0.66 {
        return Biome::TropicalSeasonalForest;
    }
    Biome::TropicalRainForest
}
impl Biome {
    fn color(self) -> image::Rgba<u8> {
        match self {
            Biome::Ocean => image::Rgba([68, 68, 122, 255]),
            Biome::Beach => image::Rgba([160, 144, 119, 255]),
            Biome::Scorched => image::Rgba([85, 85, 85, 255]),
            Biome::Tundra => image::Rgba([187, 187, 170, 255]),
            Biome::TemperateDesert => image::Rgba([201, 210, 155, 255]),
            Biome::Shrubland => image::Rgba([136, 153, 119, 255]),
            Biome::Grassland => image::Rgba([136, 170, 85, 255]),
            Biome::TemperateDeciduousForest => image::Rgba([103, 148, 89, 255]),
            Biome::TemperateRainForest => image::Rgba([68, 136, 85, 255]),
            Biome::SubtropicalDesert => image::Rgba([210, 185, 139, 255]),
            Biome::TropicalSeasonalForest => image::Rgba([85, 153, 68, 255]),
            Biome::TropicalRainForest => image::Rgba([51, 119, 85, 255]),
            Biome::Taiga => image::Rgba([153, 170, 119, 255]),
            Biome::Snow => image::Rgba([221, 221, 228, 255]),
        }
    }
}

fn chunk_generated(
    mut commands: Commands,
    mut chunks: Query<(Entity, &mut GeneratingChunk, &Chunk)>,
    mut materials: ResMut<Assets<Image>>,
) {
    for (chunk_entity, mut generating_chunk, _chunk) in chunks.iter_mut() {
        if let Some(result) =
            future::block_on(future::poll_once(&mut generating_chunk.generation_task))
                .unwrap_or(None)
        {
            //info!("Chunk Generated at {}", chunk.0);
            let mut entity_commands = commands.entity(chunk_entity);
            let mut texture = Image::from_dynamic(
                image::DynamicImage::ImageRgba8(result.texture),
                true,
                RenderAssetUsages::RENDER_WORLD,
            );
            texture.sampler = ImageSampler::nearest();
            entity_commands.insert((
                Sprite {
                    custom_size: Some(Vec2::splat(CHUNK_SIZE as f32 * SCALE)),
                    ..default()
                },
                materials.add(texture),
            ));
            entity_commands.remove::<GeneratingChunk>();
        }
    }
}

struct ChunkGenerationResult {
    texture: image::ImageBuffer<Rgba<u8>, Vec<u8>>,
}
impl ChunkGenerationResult {
    fn new(texture: image::ImageBuffer<Rgba<u8>, Vec<u8>>) -> Self {
        Self { texture }
    }
}
#[derive(Component)]
struct GeneratingChunk {
    generation_task: Task<Option<ChunkGenerationResult>>,
}

fn start_level(frames: Res<FrameCount>, mut commands: Commands) {
    if frames.0 == 75 {
        let thread_pool = AsyncComputeTaskPool::get();
        for (x, y) in spiral::ChebyshevIterator::new(0, 0, SPAWN_CHUNKS) {
            let pos = IVec2::new(x, y);
            let task = thread_pool.spawn(gen_chunk(pos));
            commands
                .spawn(Chunk { pos })
                // .insert(Collider::cuboid(
                //     CHUNK_SIZE as f32 * SCALE / 2.0,
                //     CHUNK_SIZE as f32 * SCALE / 2.0,
                // ))
                .insert(SpatialBundle::from_transform(Transform::from_translation(
                    Vec2::new(
                        (x * CHUNK_SIZE as i32) as f32 * SCALE,
                        (y * CHUNK_SIZE as i32) as f32 * SCALE,
                    )
                    .extend(0.0),
                )))
                .insert(GeneratingChunk {
                    generation_task: task,
                });
        }
    }
}
