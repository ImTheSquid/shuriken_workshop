use std::time::Duration;
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::time::FixedTimestep;
use bevy::window::PresentMode;
use rand::prelude::SliceRandom;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f64 = 1.0 / 60.0;

const SHURIKEN_UP_COLOR: Color = Color::rgba(0.5, 0.5, 0.5, 0.5);
const SHURIKEN_DOWN_COLOR: Color = Color::rgba(0.2, 0.2, 0.2, 1.0);

const LEFT_WALL: f32 = -400.0;
const RIGHT_WALL: f32 = 400.0;
const TOP_WALL: f32 = 300.0;
const BOTTOM_WALL: f32 = -300.0;

const NUM_NINJAS: u32 = 5;
const NINJA_OFFSET: f32 = 100.0;
const NINJA_SIZE: Vec3 = Vec3::new(40.0, 80.0, 1.0);

const WALL_SIZE: Vec3 = Vec3::new(RIGHT_WALL * 2.0, 200.0, 0.0);

const SHURIKEN_SIZE: Vec3 = Vec3::new(30.0, 30.0, 0.5);

const GRAVITY: f32 = 0.25;
const SHURIKEN_INIT_VELOCITY: f32 = 20.0;

const PADDLE_COLOR: Color = Color::rgb(0.0, 0.0, 1.0);
const PADDLE_Y: f32 = -40.0;
const PADDLE_SIZE: Vec3 = Vec3::new(RIGHT_WALL * 2.0 / (NUM_NINJAS as f32 * 1.2), 20.0, 1.0);

const MAX_HEALTH: usize = 3;
const HEALTH_OFFSET: f32 = 20.0;
const HEALTH_SIZE: Vec3 = Vec3::new(20.0, 10.0, 1.0);
const HEALTH_COLOR: Color = Color::rgb(0.0, 1.0, 0.0);

/// Probable difficulty equation: `y=1.1^(0.7x)/20`
#[derive(Resource)]
struct State {
    lives: usize,
    blocked: usize,
    timer: Timer,
    key_debounce: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            lives: MAX_HEALTH,
            blocked: 0,
            timer: Timer::new(Duration::from_millis((1.1_f32.powf(0.0) * 1000.0 * 1.5) as u64), TimerMode::Repeating),
            key_debounce: false,
        }
    }
}

/// Has a position indicating which indicator it is (0, 1, 2...)
#[derive(Component)]
struct HealthMarker(usize);

/// A shuriken
#[derive(Component)]
struct Shuriken;

/// A 1-D velocity
#[derive(Component)]
struct Velocity(f32);

/// A ninja that can be hit
/// Also where the shurikens can be spawned
#[derive(Component)]
struct Ninja;

/// The movable paddle to block shurikens, along with its position
#[derive(Component)]
struct Paddle(u32);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::hex("25e6ee").unwrap()))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: RIGHT_WALL * 2.0,
                height: TOP_WALL * 2.0,
                title: "Shuriken Workshop".to_string(),
                present_mode: PresentMode::AutoVsync,
                resizable: false,
                ..default()
            },
            add_primary_window: true,
            exit_on_all_closed: true,
            close_when_requested: true,
        }))
        .init_resource::<State>()
        .add_startup_system(setup)
        .add_system_set(SystemSet::new()
            .with_run_criteria(FixedTimestep::step(TIME_STEP))
            .with_system(check_collisions)
            .with_system(do_physics.before(check_collisions))
            .with_system(spawn_shurikens.after(check_collisions))
            .with_system(update_paddle.after(check_collisions))
        )
        .add_system(update_scoreboard)
        .run();
}

fn setup(
    mut commands: Commands
) {
    commands.spawn(Camera2dBundle::default());

    // Spawn the wall behind the ninjas
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.0, 0.0, 0.0),
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, BOTTOM_WALL + WALL_SIZE.y / 2.0, 0.0),
            scale: WALL_SIZE,
            ..default()
        },
        ..default()
    });

    // Spawn ninjas and shuriken spawners
    for x_pos in ((LEFT_WALL + (RIGHT_WALL / NUM_NINJAS as f32)) as i32..RIGHT_WALL as i32).step_by((RIGHT_WALL * 2.0 / NUM_NINJAS as f32) as usize) {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgb(1.0, 0.0, 0.0),
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(x_pos as f32, BOTTOM_WALL + NINJA_OFFSET, 0.0),
                    scale: NINJA_SIZE,
                    ..default()
                },
                ..default()
            },
            Ninja
        ));
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: PADDLE_COLOR,
                ..default()
            },
            transform: Transform {
                translation: Vec3::new(LEFT_WALL + RIGHT_WALL / (NUM_NINJAS as f32), PADDLE_Y, 0.0),
                scale: PADDLE_SIZE,
                ..default()
            },
            ..default()
        },
        Paddle(0)
    ));

    for (i, x_pos) in ((LEFT_WALL + (RIGHT_WALL / MAX_HEALTH as f32)) as i32..RIGHT_WALL as i32).step_by((RIGHT_WALL * 2.0 / MAX_HEALTH as f32) as usize).enumerate() {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: HEALTH_COLOR,
                    ..default()
                },
                transform: Transform {
                    translation: Vec3::new(x_pos as f32, BOTTOM_WALL + HEALTH_OFFSET, 0.0),
                    scale: HEALTH_SIZE,
                    ..default()
                },
                ..default()
            },
            HealthMarker(i),
        ));
    }
}

fn update_paddle(
    mut state: ResMut<State>,
    keyboard_input: Res<Input<KeyCode>>,
    mut paddle: Query<(&mut Paddle, &mut Transform)>
) {
    if state.key_debounce {
        if !(keyboard_input.pressed(KeyCode::Left) || keyboard_input.pressed(KeyCode::Right)) {
            state.key_debounce = false;
        }
        return;
    }

    let (mut paddle, mut transform) = paddle.single_mut();
    if keyboard_input.pressed(KeyCode::Left) && paddle.0 > 0 {
        paddle.0 -= 1;
        state.key_debounce = true;
    } else if keyboard_input.pressed(KeyCode::Right) && paddle.0 < NUM_NINJAS - 1 {
        paddle.0 += 1;
        state.key_debounce = true;
    }

    transform.translation.x = LEFT_WALL + RIGHT_WALL / (NUM_NINJAS as f32) + RIGHT_WALL * 2.0 * paddle.0 as f32 / NUM_NINJAS as f32;
}

fn do_physics(
    mut shuriken_query: Query<(&mut Sprite, &mut Transform, &mut Velocity), With<Shuriken>>
) {
    for (mut sprite, mut transform, mut velocity) in &mut shuriken_query {
        transform.translation.y += velocity.0;
        velocity.0 -= GRAVITY;
        if velocity.0 < 0.0 {
            sprite.color = SHURIKEN_DOWN_COLOR;
        }
    }
}

fn spawn_shurikens(
    mut commands: Commands,
    mut state: ResMut<State>,
    spawner_query: Query<&Transform, With<Ninja>>
) {
    state.timer.tick(Duration::from_millis((TIME_STEP * 1000.0) as u64));
    if state.timer.finished() {
        let spawners = spawner_query.iter().collect::<Vec<_>>();
        let chosen = **spawners.choose(&mut rand::thread_rng()).unwrap();
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: SHURIKEN_UP_COLOR,
                    ..default()
                },
                transform: chosen.with_scale(SHURIKEN_SIZE),
                ..default()
            },
            Shuriken,
            Velocity(SHURIKEN_INIT_VELOCITY),
        ));
        state.timer.reset();
    }
}

fn check_collisions(
    mut commands: Commands,
    mut state: ResMut<State>,
    shuriken_query: Query<(Entity, &Transform, &Velocity), With<Shuriken>>,
    ninja_query: Query<&Transform, With<Ninja>>,
    paddle_query: Query<&Transform, With<Paddle>>,
) {
    let paddle = paddle_query.single();
    for (shuriken, shuriken_pos) in shuriken_query.iter().filter(|(_, _, v)| v.0 < 0.0).map(|(e, t, _)| (e, t)) {
        if collide(shuriken_pos.translation, SHURIKEN_SIZE.truncate(), paddle.translation, PADDLE_SIZE.truncate()).is_some() {
            commands.entity(shuriken).despawn();
            state.blocked += 1;
            state.timer = Timer::new(Duration::from_millis((1.1_f32.powf(1.0 / state.blocked as f32) * 1000.0 * 1.5) as u64), TimerMode::Repeating);
            continue;
        }

        for ninja in &ninja_query {
            if collide(shuriken_pos.translation, SHURIKEN_SIZE.truncate(), ninja.translation, NINJA_SIZE.truncate()).is_some() {
                commands.entity(shuriken).despawn();
                state.lives -= 1;
            }
        }
    }
}

fn update_scoreboard(
    mut commands: Commands,
    mut exit: EventWriter<AppExit>,
    state: Res<State>,
    health_query: Query<(Entity, &HealthMarker)>
) {
    for (entity, marker) in &health_query {
        if marker.0 + 1 > state.lives {
            commands.entity(entity).despawn();
        }
    }

    if state.lives == 0 {
        println!("GAME OVER! You blocked {} shuriken(s)", state.blocked);
        exit.send(AppExit);
    }
}
