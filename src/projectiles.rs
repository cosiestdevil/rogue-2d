use std::time::Duration;

use bevy::{
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
};
use bevy_rapier2d::{
    dynamics::{ExternalImpulse, RigidBody},
    geometry::{ActiveEvents, Collider, CollisionGroups, Sensor},
    pipeline::CollisionEvent,
};
use bevy_spritesheet_animation::{
    component::SpritesheetAnimation, library::SpritesheetLibrary, spritesheet::Spritesheet,
};

use crate::{DamageBuffer, DamageSource, GameState, Health, Player};

pub struct ProjectilesPlugin;
impl Plugin for ProjectilesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_pure_projectile.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, remove_projectile.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, projectile_collide.run_if(in_state(GameState::Playing)));
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
            let rotation =
                Quat::from_axis_angle(Vec3::Z, ((player.facing + 270.0) % 360.0).to_radians());
            commands
                .spawn(Projectile {
                    lifespan: Timer::from_seconds(5.0, TimerMode::Once),
                    damage:2,
                })
                .insert(SpriteSheetBundle {
                    texture,
                    atlas: TextureAtlas {
                        layout,
                        ..default()
                    },
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(32.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(transform.translation + Vec3::Z)
                        .with_rotation(rotation),
                    ..default()
                })
                .insert(Collider::cuboid(16.0, 8.0))
                .insert(CollisionGroups::new(
                    crate::PROJECTILE_GROUP,
                    crate::ENEMY_GROUP,
                ))
                .insert(ActiveEvents::COLLISION_EVENTS)
                .insert(RigidBody::Dynamic)
                .insert(ExternalImpulse {
                    impulse: (rotation * (Vec3::X * (1024.0 * 64.0))).truncate(),
                    torque_impulse: 0.0,
                })
                .insert(SpritesheetAnimation::from_id(animation));
        }
    }
}

fn remove_projectile(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &mut Projectile)>,
    time: ResMut<Time>,
) {
    for (entity, mut projectile) in projectiles.iter_mut() {
        projectile.lifespan.tick(time.delta());
        if projectile.lifespan.just_finished() {
            commands.entity(entity).despawn_recursive();
        } 
    }
}

fn projectile_collide(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut projectile: Query<(&mut Projectile, Option<&Children>)>,
    damage_source: Query<Entity, With<DamageSource>>,
    mut other: Query<&mut DamageBuffer, With<Health>>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _flags) => {
                if let Ok((mut projectile, _)) = projectile.get_mut(*a) {
                    if let Ok(mut other) = other.get_mut(*b) {
                        let damage_entity = commands.spawn(DamageSource).id();
                        commands.entity(*b).add_child(damage_entity);
                        other.0.push(crate::Damage {
                            source: damage_entity,
                            amount: projectile.damage,
                        });
                        projectile.lifespan = Timer::new(Duration::from_millis(100), TimerMode::Once);
                    }
                } else if let Ok((mut projectile, _)) = projectile.get_mut(*b) {
                    if let Ok(mut player) = other.get_mut(*a) {
                        let damage_entity = commands.spawn(DamageSource).id();
                        commands.entity(*b).add_child(damage_entity);
                        player.0.push(crate::Damage {
                            source: damage_entity,
                            amount: projectile.damage,
                        });
                        projectile.lifespan = Timer::new(Duration::from_millis(100), TimerMode::Once);
                    }
                }
            }
            CollisionEvent::Stopped(a, b, _flags) => {
                if let Ok((_, children)) = projectile.get(*a) {
                    if other.get(*b).is_ok() {
                        if let Some(children) = children {
                            for &child in children.iter() {
                                if let Ok(source) = damage_source.get(child) {
                                    commands.entity(source).despawn_recursive();
                                }
                            }
                        }
                    }
                } else if let Ok((_, children)) = projectile.get(*b) {
                    if other.get(*a).is_ok() {
                        if let Some(children) = children {
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

#[derive(Component)]
struct Projectile {
    lifespan: Timer,
    damage: u32,
}
#[derive(Component)]
pub struct PureProjectileSkill {
    pub(crate) cooldown: Timer,
}
