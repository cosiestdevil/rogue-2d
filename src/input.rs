use bevy::prelude::*;
use bevy_ineffable::{config::simple_asset_loading::MergeMode, prelude::*};
use bevy_spritesheet_animation::component::SpritesheetAnimation;

use crate::{Player, PlayerAnimation};

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(IneffablePlugin);
        app.register_input_action::<PlayerInput>();
        app.add_systems(Startup, init);
        app.add_systems(Update, player_movement);
        app.add_systems(Update, player_rotate);
    }
}

#[derive(InputAction)]
pub enum PlayerInput {
    /// In this example, the only thing the player can do is honk.
    /// We must define what kind of input Honk is. Honking is
    /// enacted instantaneously, so we'll define it as a pulse.
    #[ineffable(dual_axis)]
    Move,
    #[ineffable(dual_axis)]
    Face,
    // You can add more actions here...
}
const SPEED: f32 = 72.0;
/// Speed at which the player is rotated.
/// Value is negative because it feels more natural.
//const ROTATE_SPEED: f32 = -3.0;
fn init(mut ineffable: IneffableCommands) {
    ineffable.load_configs(vec![(MergeMode::Base, "player.input.ron")]);
}
fn player_movement(
    mut commands: Commands,
    bindings: Res<Ineffable>,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &Player)>,
) {
    if let Ok((entity, mut transform, player)) = query.get_single_mut() {
        let movement_direction = bindings.direction_2d(ineff!(PlayerInput::Move));
        transform.translation.x += movement_direction.x * time.delta_seconds() * SPEED;
        transform.translation.y += movement_direction.y * time.delta_seconds() * SPEED;
        //let angle = Vec2::X.dot(player.facing).acos().to_degrees();
        let angle = player.facing;
        let mut player_entity = commands.entity(entity);

        let moving = movement_direction.length() > 0.0;
        let mut animation = *player
            .animations
            .get(if moving {
                &PlayerAnimation::WalkDown
            } else {
                &PlayerAnimation::IdleDown
            })
            .unwrap();
        if (45.0..135.0).contains(&angle) {
            animation = *player
                .animations
                .get(if moving {
                    &PlayerAnimation::WalkRight
                } else {
                    &PlayerAnimation::IdleRight
                })
                .unwrap();
        }
        if !(-135.0..=135.0).contains(&angle) {
            animation = *player
                .animations
                .get(if moving {
                    &PlayerAnimation::WalkUp
                } else {
                    &PlayerAnimation::IdleUp
                })
                .unwrap();
        }
        if (-135.0..-45.0).contains(&angle) {
            animation = *player
                .animations
                .get(if moving {
                    &PlayerAnimation::WalkLeft
                } else {
                    &PlayerAnimation::IdleLeft
                })
                .unwrap();
        }

        player_entity.insert(SpritesheetAnimation::from_id(animation));
    }
}

fn player_rotate(bindings: Res<Ineffable>, /*time: Res<Time>,*/ mut query: Query<&mut Player>) {
    if let Ok(mut player) = query.get_single_mut() {
        let mut direction = bindings.direction_2d(ineff!(PlayerInput::Face));

        if direction.distance(Vec2::ZERO) > 0.5 {
            //info!("{:?}", direction.distance(Vec2::ZERO));
            direction = direction.normalize_or_zero();
            player.facing = direction.x.atan2(-direction.y).to_degrees()
        };
    }
}
