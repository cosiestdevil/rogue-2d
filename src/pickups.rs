use bevy::{
    ecs::{system::CommandQueue, world},
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
};
use bevy_rapier2d::{
    control::KinematicCharacterController,
    dynamics::RigidBody,
    geometry::{ActiveCollisionTypes, ActiveEvents, Collider, CollisionGroups, Sensor},
    pipeline::CollisionEvent,
};
use bevy_spritesheet_animation::{
    animation::AnimationDuration, component::SpritesheetAnimation, library::SpritesheetLibrary,
    spritesheet::Spritesheet,
};

use crate::{GameState, Player, PICKUP_GROUP};

pub struct PickupsPlugin;

impl Plugin for PickupsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_pickup_animations);

        app.add_systems(
            Update,
            toggle_attact_pickup.run_if(in_state(GameState::Playing)),
        );
        app.add_systems(Update, attract_pickup.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, pickup_pickup.run_if(in_state(GameState::Playing)));
    }
}

fn setup_pickup_animations(mut library: ResMut<SpritesheetLibrary>) {
    let sheet = Spritesheet::new(4, 1);
    let exp_clip = library.new_clip(|clip| {
        clip.push_frame_indices(sheet.row_partial(0, 0..4));
        clip.set_default_duration(AnimationDuration::PerCycle(2500));
    });
    let exp_animation = library.new_animation(|animation| {
        animation.add_stage(exp_clip.into());
    });
    library
        .name_animation(exp_animation, EXP_ANIMATION)
        .unwrap();
}
pub const EXP_ANIMATION: &str = "experience orb";

#[derive(Component)]
pub struct Pickup;

#[derive(Component)]
pub struct ExperiencePickup {
    pub amount: u64,
}

pub fn spawn_experience_pickup(
    library: &Res<SpritesheetLibrary>,
    atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    assets: &Res<AssetServer>,
    origin: Transform,
) -> CommandQueue {
    let mut command_queue = CommandQueue::default();
    let texture = assets.load_with_settings("exp.png", |s: &mut ImageLoaderSettings| match &mut s
        .sampler
    {
        ImageSampler::Default => s.sampler = ImageSampler::nearest(),
        ImageSampler::Descriptor(sampler) => {
            *sampler = ImageSamplerDescriptor::nearest();
        }
    });
    let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        Vec2::new(128.0, 128.0),
        4,
        1,
        None,
        None,
    ));
    let animation_id = library.animation_with_name(EXP_ANIMATION).unwrap();
    command_queue.push(move |world: &mut World| {
        world
            .spawn(SpriteSheetBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(16.0)),
                    ..default()
                },
                texture: texture.clone(),
                atlas: TextureAtlas {
                    layout: layout.clone(),
                    ..default()
                },
                transform: origin,
                ..default()
            })
            .insert(Pickup)
            .insert(ExperiencePickup { amount: 100 })
            .insert(SpritesheetAnimation::from_id(animation_id))
            .insert(Collider::ball(8.0))
            .insert(RigidBody::Dynamic)
            .insert(ActiveEvents::COLLISION_EVENTS)
            .insert(CollisionGroups::new(
                PICKUP_GROUP,
                crate::PLAYER_PICKUP_GROUP|crate::PLAYER_GROUP
            ))
            .insert(KinematicCharacterController::default())
            .insert(Sensor);
    });
    command_queue
}

fn toggle_attact_pickup(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    player: Query<(), With<PlayerPickup>>,
    pickups: Query<(), (With<Pickup>, Without<PlayerPickup>)>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _flags) => {
                if player.get(*a).is_ok() && pickups.get(*b).is_ok() {
                    let mut pickup = commands.entity(*b);
                    pickup.insert(AttractedTo);
                }
                if player.get(*b).is_ok() && pickups.get(*a).is_ok() {
                    let mut pickup = commands.entity(*a);
                    pickup.insert(AttractedTo);
                }
            }
            CollisionEvent::Stopped(a, b, _flags) => {}
        }
    }
}

fn attract_pickup(
    mut pickups: Query<
        (&Transform, &mut KinematicCharacterController),
        (With<Pickup>, With<AttractedTo>, Without<Player>),
    >,
    player: Query<&Transform, With<Player>>,
    time: Res<Time>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    for (pickup_transform, mut pickup_controller) in pickups.iter_mut() {
        info!("Moving pickup!");
        let direction = (player.translation - pickup_transform.translation).normalize();
        pickup_controller.translation = Some((direction * 64.0 * time.delta_seconds()).truncate());
    }
}
fn pickup_pickup(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    player: Query<(), With<Player>>,
    pickups: Query<(), (With<Pickup>, Without<Player>)>,
) {
    for collision_event in collision_events.read() {
        match collision_event {
            CollisionEvent::Started(a, b, _flags) => {
                if player.get(*a).is_ok() && pickups.get(*b).is_ok() {
                    let mut pickup = commands.entity(*b);
                    pickup.despawn_recursive();
                }
                if player.get(*b).is_ok() && pickups.get(*a).is_ok() {
                    let mut pickup = commands.entity(*a);
                    pickup.despawn_recursive();
                }
            }
            CollisionEvent::Stopped(a, b, _flags) => {}
        }
    }
}
#[derive(Component)]
struct AttractedTo;
#[derive(Component)]
pub struct PlayerPickup;
