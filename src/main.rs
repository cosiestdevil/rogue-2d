#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(clippy::type_complexity)]

use std::time::Duration;

use bevy::{
    //core::FrameCount,
    log::LogPlugin, prelude::*, render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor}, utils::HashMap
};
use bevy_rapier2d::prelude::*;
use bevy_spritesheet_animation::{
    animation::AnimationId, component::SpritesheetAnimation, library::SpritesheetLibrary,
    plugin::SpritesheetAnimationPlugin, spritesheet::Spritesheet,
};
use projectiles::PureProjectileSkill;

mod enemies;
mod generation;
mod input;
mod pickups;
mod projectiles;
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(LogPlugin{level:bevy::log::Level::DEBUG,..default()}));
    app.add_plugins(SpritesheetAnimationPlugin);
    app.add_plugins(input::InputPlugin);
    app.add_plugins(generation::GenerationPlugin);
    app.add_plugins(projectiles::ProjectilesPlugin);
    app.add_plugins(enemies::EnemiesPlugin);
    app.add_plugins(pickups::PickupsPlugin);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(
        generation::SCALE,
    ));
    //app.add_plugins(RapierDebugRenderPlugin::default());
    app.insert_resource(RapierConfiguration {
        gravity: Vec2::ZERO,
        ..RapierConfiguration::new(1.0)
    });
    app.insert_state(GameState::StartScreen);
    app.add_systems(Startup, setup_graphics);
    app.add_systems(Startup, setup_start_screen);
    app.add_systems(OnExit(GameState::StartScreen), teardown_start_screen);
    app.add_systems(OnEnter(GameState::Playing), setup_character);
    app.add_systems(Update, apply_damage.run_if(in_state(GameState::Playing)));
    app.add_systems(Update, despawn_dead.run_if(in_state(GameState::Playing)));
    app.add_systems(Update, end_level.run_if(in_state(GameState::Playing)));
    app.add_systems(
        Update,
        update_health_bars.run_if(in_state(GameState::Playing)),
    );
    app.add_systems(Update, update_exp_bars.run_if(in_state(GameState::Playing)));
    app.add_systems(Update, level_up.run_if(in_state(GameState::Playing)));
    app.run();
}

fn setup_start_screen(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    window: Query<&Window>,
) {
    let window = window.single();

    let mut width = window.resolution.width();
    let mut height = window.resolution.height();
    if width > height {
        //Landscape
        height = width / 16.0 * 9.0;
    } else if width < height {
        //Portrait
        width = height / 9.0 * 16.0;
    }
    commands
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                width: Val::Percent(100.0),
                height: Val::Percent(100.),
                justify_items: JustifyItems::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(StartScreen)
        .with_children(|commands| {
            commands.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(width),
                    height: Val::Px(height),
                    ..default()
                },
                image: UiImage::new(asset_server.load("start_screen.png")),
                ..default()
            });
        });
}
fn teardown_start_screen(mut commands: Commands, screens: Query<Entity, With<StartScreen>>) {
    for screen in screens.iter() {
        commands.entity(screen).despawn_recursive();
    }
}
#[derive(Component)]
struct StartScreen;
#[derive(States, Debug, Default, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GameState {
    #[default]
    StartScreen,
    Playing,
    #[allow(dead_code)]
    DeathScreen,
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 0.5,
            far: 1000.,
            near: -1000.,
            ..default()
        },
        ..default()
    }); //.insert(PanCam::default());
    commands.spawn(DirectionalLightBundle::default());
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 2000.0,
    });
}
#[derive(Resource)]
struct Level {
    runtime: Timer,
}
fn end_level(time: Res<Time>, mut level: ResMut<Level>) {
    level.runtime.tick(time.delta());
}

fn setup_character(
    //frames: Res<FrameCount>,
    mut commands: Commands,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut camera: Query<Entity, With<Camera>>,
    assets: Res<AssetServer>,
) {
    //if frames.0 == 10 {
    // Create an animation
    commands.insert_resource(Level {
        runtime: Timer::from_seconds(15.0 * 60.0, TimerMode::Once),
    });

    let sheet = Spritesheet::new(13, 46);
    let idle_down_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(10, 0..2));
        clip.push_frame_indices(sheet.row_partial(10, 5..6));
        clip.set_default_duration(
            bevy_spritesheet_animation::animation::AnimationDuration::PerCycle(5000),
        );
    });
    let idle_down_animation = library.new_animation(|animation| {
        animation.add_stage(idle_down_clip.into());
    });
    let idle_right_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(11, 0..1));
        clip.push_frame_indices(sheet.row_partial(11, 8..9));
        clip.push_frame_indices(sheet.row_partial(7, 1..2));
        clip.set_default_duration(
            bevy_spritesheet_animation::animation::AnimationDuration::PerCycle(5000),
        );
    });
    let idle_right_animation = library.new_animation(|animation| {
        animation.add_stage(idle_right_clip.into());
    });
    let idle_up_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(8, 0..2));
        clip.push_frame_indices(sheet.row_partial(8, 5..6));
        clip.set_default_duration(
            bevy_spritesheet_animation::animation::AnimationDuration::PerCycle(5000),
        );
    });
    let idle_up_animation = library.new_animation(|animation| {
        animation.add_stage(idle_up_clip.into());
    });
    let idle_left_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(9, 0..1));
        clip.push_frame_indices(sheet.row_partial(9, 8..9));
        clip.push_frame_indices(sheet.row_partial(5, 1..2));
        clip.set_default_duration(
            bevy_spritesheet_animation::animation::AnimationDuration::PerCycle(5000),
        );
    });
    let idle_left_animation = library.new_animation(|animation| {
        animation.add_stage(idle_left_clip.into());
    });
    let walk_down_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(10, 0..9));
    });
    let walk_down_animation = library.new_animation(|animation| {
        animation.add_stage(walk_down_clip.into());
    });
    let walk_right_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(11, 0..9));
    });
    let walk_right_animation = library.new_animation(|animation| {
        animation.add_stage(walk_right_clip.into());
    });
    let walk_up_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(8, 0..9));
    });
    let walk_up_animation = library.new_animation(|animation| {
        animation.add_stage(walk_up_clip.into());
    });
    let walk_left_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(9, 0..9));
    });
    let walk_left_animation = library.new_animation(|animation| {
        animation.add_stage(walk_left_clip.into());
    });
    // Spawn a sprite using Bevy's built-in SpriteSheetBundle

    let texture =
        assets.load_with_settings("character.png", |s: &mut ImageLoaderSettings| match &mut s
            .sampler
        {
            ImageSampler::Default => s.sampler = ImageSampler::nearest(),
            ImageSampler::Descriptor(sampler) => {
                *sampler = ImageSamplerDescriptor::nearest();
            }
        });

    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::new(64, 64),
        13,
        46,
        None,
        None,
    ));
    let mut player = Player::default();
    player
        .animations
        .insert(PlayerAnimation::IdleDown, idle_down_animation);
    player
        .animations
        .insert(PlayerAnimation::IdleRight, idle_right_animation);
    player
        .animations
        .insert(PlayerAnimation::IdleUp, idle_up_animation);
    player
        .animations
        .insert(PlayerAnimation::IdleLeft, idle_left_animation);
    player
        .animations
        .insert(PlayerAnimation::WalkDown, walk_down_animation);
    player
        .animations
        .insert(PlayerAnimation::WalkRight, walk_right_animation);
    player
        .animations
        .insert(PlayerAnimation::WalkUp, walk_up_animation);
    player
        .animations
        .insert(PlayerAnimation::WalkLeft, walk_left_animation);
    player.next_level = 500;
    let player_id = commands
        .spawn((
            player,
            SpriteBundle {
                texture,
                transform: Transform::from_xyz(0., 0., 2.),
                ..default()
            },
            TextureAtlas {
                layout,
                ..default()
            },
            Collider::cuboid(16.0, 32.0),
            Sensor,
            Health {
                current: 20,
                max: 20,
                invulnerability_timer: None,
                invulnerability_duration: Duration::from_secs(2),
            },
            CollisionGroups::new(PLAYER_GROUP, ENEMY_GROUP | crate::PICKUP_GROUP),
            DamageBuffer::default(),
            // Add a SpritesheetAnimation component that references our newly created animation
            SpritesheetAnimation::from_id(idle_down_animation),
            projectiles::PureProjectileSkill {
                cooldown: Timer::from_seconds(1.0, TimerMode::Repeating),
            },
        ))
        .insert(ActiveEvents::COLLISION_EVENTS)
        .id();
    commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(Vec3::ZERO)),
            Collider::cuboid(12.0, 28.0),
            CollisionGroups::new(PLAYER_GROUP, ENEMY_GROUP),
        ))
        .set_parent(player_id);
    commands
        .spawn((
            TransformBundle::from_transform(Transform::from_translation(Vec3::ZERO)),
            Collider::ball(48.0),
            Sensor,
            pickups::PlayerPickup,
            ActiveEvents::COLLISION_EVENTS,
            CollisionGroups::new(PLAYER_PICKUP_GROUP, PICKUP_GROUP),
        ))
        .set_parent(player_id);

    let _health_background = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 8.0)),
                ..default()
            },
            texture: assets.load("bars/background.png"),
            transform: Transform::from_xyz(0., -32.0, 3.),
            ..default()
        })
        .set_parent(player_id)
        .id();
    let _health_foreground = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 8.0)),
                ..default()
            },
            texture: assets.load("bars/health_foreground.png"),
            transform: Transform::from_xyz(0., -32.0, 3.2),
            ..default()
        })
        .set_parent(player_id)
        .id();
    let _health_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 8.0)),
                color: Color::linear_rgba(0.8, 0.0, 0.0, 1.0),
                ..default()
            },
            texture: assets.load("bars/bar.png"),
            transform: Transform::from_xyz(0., -32.0, 3.1),
            ..default()
        })
        .insert(HealthBar(32.0))
        .set_parent(player_id)
        .id();
    let _exp_background = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 8.0)),
                ..default()
            },
            texture: assets.load("bars/background.png"),
            transform: Transform::from_xyz(0., -38.0, 3.),
            ..default()
        })
        .set_parent(player_id)
        .id();
    let _exp_foreground = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(32.0, 8.0)),
                ..default()
            },
            texture: assets.load("bars/exp_foreground.png"),
            transform: Transform::from_xyz(0., -38.0, 3.2),
            ..default()
        })
        .set_parent(player_id)
        .id();
    let _exp_bar = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(0.0, 8.0)),
                color: Color::linear_rgba(0., 0.1, 0.67, 1.0),
                ..default()
            },
            texture: assets.load("enemy_health_bars_2.0/enemy_mana_bar_001.png"),
            transform: Transform::from_xyz(0., -38.0, 3.1),
            ..default()
        })
        .insert(ExpBar(32.0))
        .set_parent(player_id)
        .id();
    if let Ok(camera_entity) = camera.get_single_mut() {
        let mut camera = commands.entity(camera_entity);
        camera.set_parent(player_id);
    }
    //}
}
const PLAYER_GROUP: Group = Group::GROUP_1;
const PROJECTILE_GROUP: Group = Group::GROUP_2;
const ENEMY_GROUP: Group = Group::GROUP_3;
const PICKUP_GROUP: Group = Group::GROUP_4;
const PLAYER_PICKUP_GROUP: Group = Group::GROUP_5;

#[derive(Component)]
struct HealthBar(f32);

fn update_health_bars(mut bars: Query<(&HealthBar, &mut Sprite, &Parent)>, health: Query<&Health>) {
    for (bar, mut sprite, parent) in bars.iter_mut() {
        let Ok(health) = health.get(parent.get()) else {
            return;
        };
        sprite.custom_size = Some(Vec2::new(
            bar.0 * (health.current as f32 / health.max as f32),
            sprite.custom_size.unwrap().y,
        ))
    }
}

#[derive(Component)]
struct ExpBar(f32);

fn update_exp_bars(mut bars: Query<(&ExpBar, &mut Sprite, &Parent)>, health: Query<&Player>) {
    for (bar, mut sprite, parent) in bars.iter_mut() {
        let Ok(health) = health.get(parent.get()) else {
            return;
        };
        sprite.custom_size = Some(Vec2::new(
            bar.0 * (health.experience as f32 / health.next_level as f32),
            sprite.custom_size.unwrap().y,
        ))
    }
}

#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
    invulnerability_timer: Option<Timer>,
    invulnerability_duration: Duration,
}
#[derive(Component, Default, Debug)]
struct DamageBuffer(Vec<Damage>);
#[derive(Debug)]
struct Damage {
    source: Entity,
    amount: u32,
}
#[derive(Component)]
struct DamageSource;
fn apply_damage(
    mut commands: Commands,
    mut query: Query<(&mut DamageBuffer, &mut Health)>,
    time: Res<Time>,
) {
    for (mut buffer, mut health) in query.iter_mut() {
        if let Some(ref mut invuln) = &mut health.invulnerability_timer {
            if !invuln.finished() {
                invuln.tick(time.delta());
                continue;
            }
        }
        let mut took_damage = false;
        buffer.0.retain_mut(|damage| {
            if commands.get_entity(damage.source).is_some() {
                took_damage = true;
                info!("Taking {}", damage.amount);
                health.current = health.current.saturating_sub(damage.amount);
                true
            } else {
                false
            }
        });
        if took_damage && health.invulnerability_duration > Duration::ZERO {
            health.invulnerability_timer =
                Some(Timer::new(health.invulnerability_duration, TimerMode::Once));
        }
    }
}

#[derive(Component)]
struct Dead {
    timer: Timer,
}

#[derive(Component)]
struct Hurt {
    timer: Timer,
}

fn despawn_dead(mut commands: Commands, mut dead: Query<(Entity, &mut Dead)>, time: Res<Time>) {
    for (entity, mut dead) in dead.iter_mut() {
        dead.timer.tick(time.delta());
        if dead.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn level_up(mut player: Query<(&mut Player, &mut PureProjectileSkill)>) {
    if let Ok((mut player, mut skill)) = player.get_single_mut() {
        if player.experience >= player.next_level {
            player.experience = player.experience.saturating_sub(player.next_level);
            skill.cooldown = Timer::from_seconds(
                skill.cooldown.duration().as_secs_f32() * 0.8,
                TimerMode::Repeating,
            );
        }
    }
}

#[derive(Component, Default)]
pub struct Player {
    facing: f32,
    animations: HashMap<PlayerAnimation, AnimationId>,
    pub experience: u64,
    pub next_level: u64,
}

#[derive(PartialEq, Eq, PartialOrd, Hash)]
enum PlayerAnimation {
    IdleRight,
    IdleLeft,
    IdleDown,
    IdleUp,
    WalkRight,
    WalkLeft,
    WalkDown,
    WalkUp,
}
