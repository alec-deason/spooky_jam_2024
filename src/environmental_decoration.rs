use std::time::Duration;

use bevy::{
    prelude::*,
    time::common_conditions::on_timer,
};
use blenvy::*;

use crate::CLOUDS;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct FoundationIdle;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Cloud;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Star;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Water;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub enum Sky {
    Color(Color, f32),
    Transition {
        start_color: Color,
        end_color: Color,
        start_time: std::time::Duration,
        end_time: std::time::Duration,
        start_star_brightness: f32,
        end_star_brightness: f32,
    },
}

impl Sky {
    pub fn to_night(&mut self, now: std::time::Duration) {
        let color = self.current_color(now);
        *self = Sky::Transition {
            start_color: color,
            end_color: Color::srgba(0.072, 0.071, 0.072, 1.0),
            start_time: now,
            end_time: now + std::time::Duration::from_secs(2),
            start_star_brightness: self.current_star_brightness(now),
            end_star_brightness: 1.0,
        };
    }

    pub fn current_color(&self, now: std::time::Duration) -> Color {
        match self {
            Sky::Color(color, _) => *color,
            Sky::Transition {
                start_color,
                end_color,
                start_time,
                end_time,
                ..
            } => {
                let t = now - *start_time;
                let f = t.as_secs_f32() / (*end_time - *start_time).as_secs_f32();
                start_color.mix(end_color, f)
            }
        }
    }

    pub fn current_star_brightness(&self, now: std::time::Duration) -> f32 {
        match self {
            Sky::Color(_color, star_brightness) => *star_brightness,
            Sky::Transition {
                start_star_brightness,
                end_star_brightness,
                start_time,
                end_time,
                ..
            } => {
                let t = now - *start_time;
                let f = t.as_secs_f32() / (*end_time - *start_time).as_secs_f32();
                start_star_brightness + (end_star_brightness - start_star_brightness) * f
            }
        }
    }
}

pub struct EnvironmentalDecorationPlugin;

impl Plugin for EnvironmentalDecorationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FoundationIdle>()
            .register_type::<Sky>()
            .register_type::<Star>()
            .register_type::<Water>()
            .add_systems(
                Update,
                (maintain_clouds.run_if(on_timer(Duration::from_millis(1000))), move_clouds.run_if(on_timer(Duration::from_millis(16))), water_animation_control, star_animation, sky_color_animation),
            );
    }
}
fn water_animation_control(
    animations: Query<(&BlueprintAnimationPlayerLink, &BlueprintAnimations), With<FoundationIdle>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (link, animations) in animations.iter() {
        let (mut animation_player, mut transition) = animation_players.get_mut(link.0).unwrap();
        if let Some(animation) = animations.named_indices.get("Idle") {
            if !animation_player.is_playing_animation(*animation) {
                transition
                    .play(&mut animation_player, *animation, std::time::Duration::ZERO)
                    .repeat();
            }
        }
    }
}

fn sky_color_animation(
    mut query: Query<(&mut Sky, &Handle<StandardMaterial>), Without<Star>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut sky_state, material_handle) in &mut query {
        if let Some(material) = materials.get_mut(material_handle) {
            match &*sky_state {
                Sky::Color(color, _) => material.emissive = (*color).into(),
                Sky::Transition {
                    end_color,
                    end_time,
                    end_star_brightness,
                    ..
                } => {
                    let now = time.elapsed();
                    if *end_time <= now {
                        *sky_state = Sky::Color(*end_color, *end_star_brightness);
                    }
                    material.emissive = sky_state.current_color(now).into();
                }
            }
        }
    }
}

fn star_animation(
    mut query: Query<(&mut Sky, &Handle<StandardMaterial>), With<Star>>,
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed();
    for (mut sky_state, material_handle) in &mut query {
        if let Some(material) = materials.get_mut(material_handle) {
            let b = match &*sky_state {
                Sky::Color(_color, star_brightness) => *star_brightness,
                Sky::Transition {
                    end_color,
                    end_time,
                    end_star_brightness,
                    ..
                } => {
                    if *end_time <= now {
                        *sky_state = Sky::Color(*end_color, *end_star_brightness);
                    }
                    sky_state.current_star_brightness(now)
                }
            };
            let c = sky_state.current_color(now).mix(&Color::WHITE, b);
            material.emissive = c.into();
            material.base_color =
                Color::srgba(0.0, 0.0, 0.0, sky_state.current_star_brightness(now));
            material.alpha_mode = AlphaMode::Blend;
        }
    }
}

fn maintain_clouds(
    mut commands: Commands,
    query: Query<(Entity, &Transform), With<Cloud>>,
) {
    let mut count = 0;
    for (entity, transform) in &query {
        if transform.translation.x > 34.0 {
            commands.entity(entity).despawn_recursive();
        } else {
            count += 1;
        }
    }

    if count < 10 && fastrand::f32() > 0.3 {
        let mut scale = Vec3::splat(fastrand::u32(90..120) as f32 / 100.0);
        scale.x = fastrand::u32(100..250) as f32 / 100.0;
        let transform = Transform::from_translation(Vec3::new(fastrand::i32(-60..-40) as f32, fastrand::i32(0..23) as f32, -7.0)).with_scale(scale);
        let path = CLOUDS[fastrand::usize(0..CLOUDS.len())];
        commands.spawn((
            transform,
            BlueprintInfo::from_path(path),
            SpawnBlueprint,
            HideUntilReady,
            GameWorldTag,
            Cloud,
        ));
    }
}

fn move_clouds(
    mut query: Query<&mut Transform, With<Cloud>>,
) {
    for mut transform in &mut query {
        transform.translation.x += 0.1 + 0.05*(transform.translation.y/23.0);
    }
}
