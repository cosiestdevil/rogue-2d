use std::time::Duration;

use bevy::{
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    utils::HashMap,
};
use bevy_rapier2d::{
    control::KinematicCharacterController,
    dynamics::{LockedAxes, RigidBody},
    geometry::{Collider, CollisionGroups, Restitution},
    pipeline::CollisionEvent,
};
use bevy_spritesheet_animation::{
    animation::AnimationId, component::SpritesheetAnimation, library::SpritesheetLibrary,
    spritesheet::Spritesheet,
};
use rand::{thread_rng, Rng};

use crate::{
    DamageBuffer, DamageSource, GameState, Health, Level, Player, ENEMY_GROUP, PLAYER_GROUP,
    PROJECTILE_GROUP,
};
pub struct EnemiesPlugin;
impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_slime.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, move_slime.run_if(in_state(GameState::Playing)));
        app.add_systems(
            Update,
            slime_hurt_player.run_if(in_state(GameState::Playing)),
        );
        app.insert_resource(SlimeSpawn {
            cooldown: Timer::from_seconds(6.0, TimerMode::Once),
            cooldown_func: |time| {
                let delay = (time.as_secs_f32() / 150.0).cos() * 5.0;
                Duration::from_secs_f32(delay)
            },
        });
    }
}

#[derive(Resource)]
struct SlimeSpawn {
    cooldown: Timer,
    cooldown_func: fn(Duration) -> Duration,
}

fn spawn_slime(
    mut commands: Commands,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    assets: Res<AssetServer>,
    player: Query<&Transform, With<Player>>,
    time: Res<Time>,
    mut slime_spawn: ResMut<SlimeSpawn>,
    level: Res<Level>,
) {
    // Space was pressed
    slime_spawn.cooldown.tick(time.delta());
    if !slime_spawn.cooldown.just_finished() {
        return;
    }
    slime_spawn.cooldown = Timer::new(
        (slime_spawn.cooldown_func)(level.runtime.elapsed()),
        TimerMode::Once,
    );
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
    let player_translation = player.single().translation;
    let mut origin = player_translation;
    let offset_x: f32 = thread_rng().gen_range(-256.0..256.0);
    let offset_y: f32 = thread_rng().gen_range(-256.0..256.0);
    origin.x += offset_x;
    origin.y += offset_y;
    if player_translation.distance(origin) < 32.0 {
        origin += (origin - player_translation).normalize() * 32.0
    }
    let mut slime = Slime {
        damage: 1,
        ..default()
    };
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
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(Restitution::coefficient(2.0))
        .insert(RigidBody::Dynamic)
        .insert(Health {
            current: 2,
            max: 2,
            invulnerability_timer: None,
            invulnerability_duration: Duration::ZERO,
        })
        .insert(DamageBuffer::default())
        .insert(CollisionGroups::new(
            ENEMY_GROUP,
            PLAYER_GROUP | PROJECTILE_GROUP,
        ))
        .insert(KinematicCharacterController::default())
        .insert(SpritesheetAnimation::from_id(animation));
}
fn move_slime(
    mut commands: Commands,
    mut slimes: Query<
        (
            Entity,
            &Transform,
            &mut KinematicCharacterController,
            &Slime,
        ),
        (With<Slime>, Without<Player>),
    >,
    player: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    for (slime_entity, slime_transform, mut slime_controller, slime) in slimes.iter_mut() {
        let direction = (player.translation - slime_transform.translation).normalize();
        slime_controller.translation = Some((direction * 32.0 * time.delta_seconds()).truncate());
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
    damage: u32,
    animations: HashMap<SlimeAnimation, AnimationId>,
}

fn slime_hurt_player(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    slime: Query<(&Slime, Option<&Children>)>,
    damage_source: Query<Entity, With<DamageSource>>,
    mut player: Query<&mut DamageBuffer, With<Player>>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _flags) => {
                if let Ok((slime, _)) = slime.get(*a) {
                    if let Ok(mut player) = player.get_mut(*b) {
                        // info!("Slime Started Colliding With Player");
                        let damage_entity = commands.spawn(DamageSource).id();
                        commands.entity(*a).add_child(damage_entity);
                        player.0.push(crate::Damage {
                            source: damage_entity,
                            amount: slime.damage,
                        });
                    }
                } else if let Ok((slime, _)) = slime.get(*b) {
                    if let Ok(mut player) = player.get_mut(*a) {
                        // info!("Slime Started Colliding With Player");
                        let damage_entity = commands.spawn(DamageSource).id();
                        commands.entity(*b).add_child(damage_entity);
                        player.0.push(crate::Damage {
                            source: damage_entity,
                            amount: slime.damage,
                        });
                    }
                }
            }
            CollisionEvent::Stopped(a, b, _flags) => {
                if let Ok((_, children)) = slime.get(*a) {
                    if player.get(*b).is_ok() {
                        //info!("Slime Stopped Colliding With Player");
                        if let Some(children) = children {
                            // info!("Slime Had Children");
                            for &child in children.iter() {
                                if let Ok(source) = damage_source.get(child) {
                                    commands.entity(source).despawn_recursive();
                                }
                            }
                        }
                    }
                } else if let Ok((_, children)) = slime.get(*b) {
                    if player.get(*a).is_ok() {
                        // info!("Slime Stopped Colliding With Player");
                        if let Some(children) = children {
                            // info!("Slime Had Children");
                            for &child in children.iter() {
                                if let Ok(source) = damage_source.get(child) {
                                    commands.entity(source).despawn_recursive();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Hash)]
enum SlimeAnimation {
    Idle,
    WalkRight,
    WalkLeft,
}
