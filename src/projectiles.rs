use bevy::{
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
};
use bevy_spritesheet_animation::{
    component::SpritesheetAnimation, library::SpritesheetLibrary, spritesheet::Spritesheet,
};

use crate::Player;

pub struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_pure_projectile);
        app.add_systems(Update, move_projectile);
    }
}

fn spawn_pure_projectile(
    mut commands: Commands,
    mut player: Query<(&Player, &mut PureProjectileSkill, &Transform)>,
    time: Res<Time>,
    assets: Res<AssetServer>,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for (player, mut skill, transform) in player.iter_mut() {
        skill.cooldown.tick(time.delta());
        if skill.cooldown.finished() {
            let texture = assets.load_with_settings(
                "projectiles/pure/spritesheet.png",
                |s: &mut ImageLoaderSettings| match &mut s.sampler {
                    ImageSampler::Default => s.sampler = ImageSampler::nearest(),
                    ImageSampler::Descriptor(sampler) => {
                        *sampler = ImageSamplerDescriptor::nearest();
                    }
                },
            );
            let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
                Vec2::new(200.0, 200.0),
                5,
                5,
                None,
                None,
            ));
            let sheet = Spritesheet::new(5, 5);
            let clip = library.new_clip(|clip| {
                clip.push_frame_indices(sheet.row_partial(0, 0..5));
            });
            let animation = library.new_animation(|animation| {
                animation.add_stage(clip.into());
            });
            let rotation = Quat::from_axis_angle(Vec3::Z, ((player.facing+270.0) % 360.0).to_radians());
            commands
                .spawn(Projectile {
                    rotation,
                    lifespan: Timer::from_seconds(5.0, TimerMode::Once),
                })
                .insert(SpriteSheetBundle {
                    texture,
                    atlas: TextureAtlas {
                        layout,
                        ..default()
                    },
                    sprite:Sprite{custom_size:Some(Vec2::splat(32.0)),..default()},
                    transform: Transform::from_translation(transform.translation + Vec3::Z)
                        .with_rotation(rotation),
                    ..default()
                })
                .insert(SpritesheetAnimation::from_id(animation));
        }
    }
}
fn move_projectile(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile, &mut Transform)>,
    time: ResMut<Time>,
) {
    for (entity, mut projectile, mut transform) in projectiles.iter_mut() {
        projectile.lifespan.tick(time.delta());
        if projectile.lifespan.just_finished() {
            commands.entity(entity).despawn_recursive();
        } else {
            let a = projectile.rotation * (Vec3::X);
            transform.translation += a * 128.0 * time.delta_seconds();
        }
    }
}
#[derive(Component)]
struct Projectile {
    rotation: Quat,
    lifespan: Timer,
}
#[derive(Component)]
pub struct PureProjectileSkill {
    pub(crate) cooldown: Timer,
}
