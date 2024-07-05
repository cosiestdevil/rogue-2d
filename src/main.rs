use bevy::{
    core::FrameCount,
    input::{common_conditions::input_just_pressed, keyboard::KeyboardInput},
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    utils::HashMap,
};
use bevy_rapier2d::prelude::*;
use bevy_spritesheet_animation::{
    animation::AnimationId, component::SpritesheetAnimation, library::SpritesheetLibrary,
    plugin::SpritesheetAnimationPlugin, spritesheet::Spritesheet,
};
use rand::{thread_rng, Rng};

mod generation;
mod input;
mod projectiles;
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    app.add_plugins(SpritesheetAnimationPlugin);
    app.add_plugins(input::InputPlugin);
    app.add_plugins(generation::GenerationPlugin);
    app.add_plugins(projectiles::ProjectilesPlugin);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(generation::SCALE));
    app.add_plugins(RapierDebugRenderPlugin::default());
    app.insert_resource(RapierConfiguration {
        gravity: Vec2::ZERO,
        ..RapierConfiguration::new(1.0)
    });
    app.add_systems(Startup, setup_graphics);
    app.add_systems(Update, setup_character);

    app.add_systems(
        Update,
        spawn_slime.run_if(input_just_pressed(KeyCode::Space)),
    );
    app.run();
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

fn setup_character(
    frames: Res<FrameCount>,
    mut commands: Commands,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut camera: Query<Entity, With<Camera>>,
    assets: Res<AssetServer>,
) {
    if frames.0 == 10 {
        // Create an animation
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
            Vec2::new(64.0, 64.0),
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
        let player_id = commands
            .spawn((
                player,
                SpriteSheetBundle {
                    texture,
                    atlas: TextureAtlas {
                        layout,
                        ..default()
                    },
                    transform: Transform::from_xyz(0., 0., 1.),
                    ..default()
                },
                Collider::cuboid(16.0, 32.0),
                CollisionGroups::new(PLAYER_GROUP,ENEMY_GROUP),
                // Add a SpritesheetAnimation component that references our newly created animation
                SpritesheetAnimation::from_id(idle_down_animation),
                projectiles::PureProjectileSkill {
                    cooldown: Timer::from_seconds(5.0, TimerMode::Repeating),
                },
            ))
            .id();
        if let Ok(camera_entity) = camera.get_single_mut() {
            let mut camera = commands.entity(camera_entity);
            camera.set_parent(player_id);
        }
    }
}
const PLAYER_GROUP:Group = Group::GROUP_1;
const PROJECTILE_GROUP:Group = Group::GROUP_2;
const ENEMY_GROUP:Group = Group::GROUP_3;
fn spawn_slime(
    mut commands: Commands,
    mut library: ResMut<SpritesheetLibrary>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    assets: Res<AssetServer>,
    player:Query<&Transform,With<Player>>
) {
    // Space was pressed

    let texture = assets.load_with_settings(
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
        5,
        None,
        None,
    ));
    let sheet = Spritesheet::new(6, 5);
    let clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(0, 0..6));
    });
    let animation = library.new_animation(|animation| {
        animation.add_stage(clip.into());
    });
    let mut origin = player.single().translation;
    let offset_x:f32 = thread_rng().gen_range(-256.0..256.0);
    let offset_y:f32 = thread_rng().gen_range(-256.0..256.0);
    origin.x += offset_x;
    origin.y += offset_y;
    commands
        .spawn(Slime)
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
        .insert(CollisionGroups::new(ENEMY_GROUP,PLAYER_GROUP|PROJECTILE_GROUP))
        .insert(SpritesheetAnimation::from_id(animation));
}

#[derive(Component)]
struct Slime;

#[derive(Component)]
struct Health(u32);

#[derive(Component, Default)]
struct Player {
    facing: f32,
    animations: HashMap<PlayerAnimation, AnimationId>,
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
