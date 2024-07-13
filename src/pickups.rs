use bevy::{
    ecs::world::CommandQueue,
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
};
use bevy_rapier2d::{
    control::KinematicCharacterController,
    dynamics::RigidBody,
    geometry::{ActiveEvents, Collider, CollisionGroups, Sensor},
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
pub struct Pickup {
    action: fn(&mut Player),
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
        UVec2::new(128, 128),
        4,
        1,
        None,
        None,
    ));
    let animation_id = library.animation_with_name(EXP_ANIMATION).unwrap();
    command_queue.push(move |world: &mut World| {
        world
            .spawn(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(16.0)),
                    ..default()
                },
                texture: texture.clone(),
                transform: origin,
                ..default()
            })
            .insert(TextureAtlas {
                layout: layout.clone(),
                ..default()
            })
            .insert(Pickup {
                action: |player| {
                    player.experience += 100;
                },
            })
            .insert(SpritesheetAnimation::from_id(animation_id))
            .insert(Collider::ball(8.0))
            .insert(RigidBody::Dynamic)
            .insert(ActiveEvents::COLLISION_EVENTS)
            .insert(CollisionGroups::new(
                PICKUP_GROUP,
                crate::PLAYER_PICKUP_GROUP | crate::PLAYER_GROUP,
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
        if let CollisionEvent::Started(a, b, _flags) = collision_event {
            if player.get(*a).is_ok() && pickups.get(*b).is_ok() {
                let mut pickup = commands.entity(*b);
                pickup.try_insert(AttractedTo);
            }
            if player.get(*b).is_ok() && pickups.get(*a).is_ok() {
                let mut pickup = commands.entity(*a);
                pickup.try_insert(AttractedTo);
            }
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
        // info!("Moving pickup!");
        let direction = (player.translation - pickup_transform.translation).normalize();
        pickup_controller.translation = Some((direction * 64.0 * time.delta_seconds()).truncate());
    }
}
fn pickup_pickup(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut player: Query<&mut Player>,
    pickups: Query<&Pickup, Without<Player>>,
) {
    for collision_event in collision_events.read() {
        if let CollisionEvent::Started(a, b, _flags) = collision_event {
            if let Ok(mut player) = player.get_mut(*a) {
                if let Ok(pickup) = pickups.get(*b) {
                    (pickup.action)(&mut player);
                    commands.entity(*b).despawn_recursive();
                }
            }
            if let Ok(mut player) = player.get_mut(*b) {
                if let Ok(pickup) = pickups.get(*a) {
                    (pickup.action)(&mut player);
                    commands.entity(*a).despawn_recursive();
                }
            }
        }
    }
}
#[derive(Component)]
struct AttractedTo;
#[derive(Component)]
pub struct PlayerPickup;
