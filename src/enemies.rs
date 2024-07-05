use std::time::Duration;

use bevy::{prelude::*, render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, utils::HashMap};
use bevy_rapier2d::geometry::{Collider, CollisionGroups};
use bevy_spritesheet_animation::{animation::AnimationId, component::SpritesheetAnimation, library::SpritesheetLibrary, spritesheet::Spritesheet};
use rand::{thread_rng, Rng};

use crate::{Health, Player,  ENEMY_GROUP, PLAYER_GROUP, PROJECTILE_GROUP};
pub struct EnemiesPlugin;
impl Plugin for EnemiesPlugin{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spawn_slime/* .run_if(input_just_pressed(KeyCode::Space))*/,
        );
        app.add_systems(Update, move_slime);
        app.insert_resource(SlimeSpawn{
            cooldown:Timer::from_seconds(2.0, TimerMode::Once),
            cooldown_func:|time|{
                let delay = (300.0 - time.as_secs_f32()).powf(0.3) * 0.1;
                info!(delay);
                Duration::from_secs_f32(delay)
            }
        });
    }
}

#[derive(Resource)]
struct SlimeSpawn{
    cooldown:Timer,
    cooldown_func:fn(Duration)->Duration
}


fn spawn_slime(
    mut commands: Commands,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    assets: Res<AssetServer>,
    player: Query<&Transform, With<Player>>,
    time:Res<Time>,
    mut slime_spawn:ResMut<SlimeSpawn>

) {
    // Space was pressed
    slime_spawn.cooldown.tick(time.delta());
    if !slime_spawn.cooldown.just_finished(){
        return;
    }
    slime_spawn.cooldown = Timer::new((slime_spawn.cooldown_func)(time.elapsed()),TimerMode::Once);
    let texture =
        assets.load_with_settings(
            "enemies/Slime.png",
            |s: &mut ImageLoaderSettings| match &mut s.sampler {
                ImageSampler::Default => s.sampler = ImageSampler::nearest(),
                ImageSampler::Descriptor(sampler) => {
                    *sampler = ImageSamplerDescriptor::nearest();
                }
            },
        );
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::new(100.0, 100.0),
        6,
        6,
        None,
        None,
    ));
    let sheet = Spritesheet::new(6, 6);
    let clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(0, 0..6));
    });
    let animation = library.new_animation(|animation| {
        animation.add_stage(clip.into());
    });
    let walk_left_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(2, 0..6));
    });
    let walk_left_animation = library.new_animation(|animation| {
        animation.add_stage(walk_left_clip.into());
    });
    let walk_right_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(1, 0..6));
    });
    let walk_right_animation = library.new_animation(|animation| {
        animation.add_stage(walk_right_clip.into());
    });
    let mut origin = player.single().translation;
    let offset_x: f32 = thread_rng().gen_range(-256.0..256.0);
    let offset_y: f32 = thread_rng().gen_range(-256.0..256.0);
    origin.x += offset_x;
    origin.y += offset_y;
    let mut slime = Slime::default();
    slime.animations.insert(SlimeAnimation::Idle, animation);
    slime
        .animations
        .insert(SlimeAnimation::WalkLeft, walk_left_animation);
    slime
        .animations
        .insert(SlimeAnimation::WalkRight, walk_right_animation);
    commands
        .spawn(slime)
        .insert(SpriteSheetBundle {
            texture,
            atlas: TextureAtlas {
                layout,
                ..default()
            },
            transform: Transform::from_translation(origin),
            ..default()
        })
        .insert(Collider::cuboid(16.0, 16.0))
        .insert(Health(2))
        .insert(CollisionGroups::new(
            ENEMY_GROUP,
            PLAYER_GROUP | PROJECTILE_GROUP,
        ))
        .insert(SpritesheetAnimation::from_id(animation));
}
fn move_slime(
    mut commands: Commands,
    mut slimes: Query<(Entity, &mut Transform, &Slime), (With<Slime>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    for (slime_entity, mut slime_transform, slime) in slimes.iter_mut() {
        let direction = (player.translation - slime_transform.translation).normalize();
        slime_transform.translation += direction * 96.0 * time.delta_seconds();
        let moving = direction.length() > 0.0;
        let animation = if moving {
            if direction.x > 0.0 {
                &SlimeAnimation::WalkRight
            } else {
                &SlimeAnimation::WalkLeft
            }
        } else {
            &SlimeAnimation::Idle
        };
        let Some(mut slime_entity) = commands.get_entity(slime_entity) else {
            return;
        };
        slime_entity.try_insert(SpritesheetAnimation::from_id(
            *slime.animations.get(animation).unwrap(),
        ));
    }
}
#[derive(Component, Default)]
struct Slime {
    animations: HashMap<SlimeAnimation, AnimationId>,
}

#[derive(PartialEq, Eq, PartialOrd, Hash)]
enum SlimeAnimation {
    Idle,
    WalkRight,
    WalkLeft,
}