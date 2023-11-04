use std::f32::consts::PI;

use bevy::{
    prelude::*, sprite::collide_aabb::collide,
    window::PrimaryWindow, render::texture::ImageSampler,
};
use bevy_prng::ChaCha8Rng;
use bevy_rand::{prelude::*, resource::GlobalEntropy};
use rand_core::RngCore;

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.

const WALL_THICKNESS: f32 = 10.0;
const LEFT_WALL: f32 = -450.;
const RIGHT_WALL: f32 = 450.;
const BOTTOM_WALL: f32 = -300.;
const TOP_WALL: f32 = 300.;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);
const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

const GRAVITY: f32 = 9.82 * 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EntropyPlugin::<ChaCha8Rng>::default())
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_event::<CollisionEvent>()
        // Configure how frequently our gameplay systems are run
        .insert_resource(FixedTime::new_from_secs(1.0 / 60.0))
        .add_systems(Startup, setup)
        // Add our gameplay simulation systems to the fixed timestep schedule
        .add_systems(
            FixedUpdate,
            (
                check_for_collisions,
                apply_velocity.before(check_for_collisions),
                apply_gravity.after(apply_velocity),
                //play_collision_sound.after(check_for_collisions),
            ),
        )
        .add_systems(
            Update,
            (handle_mouse, rotate_arrows, spritemap_fix, update_scoreboard, bevy::window::close_on_esc),
        )
        .run();
}

#[derive(Component)]
struct Monkey;

#[derive(Component)]
struct Arrow;

#[derive(Component)]
struct Balloon;

#[derive(Component)]
struct Falling;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Event, Default)]
struct CollisionEvent;

#[derive(Resource)]
struct CollisionSound(Handle<AudioSource>);

// This bundle is a collection of the components that define a "wall" in our game
#[derive(Bundle)]
struct WallBundle {
    // You can nest bundles inside of other bundles like this
    // Allowing you to compose their functionality
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

/// Which side of the arena is this wall located on?
enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
}

impl WallLocation {
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL, 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL, 0.),
            WallLocation::Bottom => Vec2::new(0., BOTTOM_WALL),
            WallLocation::Top => Vec2::new(0., TOP_WALL),
        }
    }

    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM_WALL;
        let arena_width = RIGHT_WALL - LEFT_WALL;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + WALL_THICKNESS)
            }
            WallLocation::Bottom | WallLocation::Top => {
                Vec2::new(arena_width + WALL_THICKNESS, WALL_THICKNESS)
            }
        }
    }
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: location.position().extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surprising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
        }
    }
}

// This resource tracks the game's score
#[derive(Resource)]
struct Scoreboard {
    score: usize,
}

// Add the game's entities to our world
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rng: ResMut<GlobalEntropy<ChaCha8Rng>>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Sound
    let ball_collision_sound = asset_server.load("sounds/laser.ogg");
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Monkey
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            texture: asset_server.load("textures/monkey.png"),
            transform: Transform {
                translation: Vec3::new(LEFT_WALL + 120.0, 60.0, 0.0),
                scale: Vec3::new(128.0, 128.0, 0.0),
                ..default()
            },
            ..default()
        },
        Monkey,
        Collider,
    ));

    // Scoreboard
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: TEXT_COLOR,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size: SCOREBOARD_FONT_SIZE,
                color: SCORE_COLOR,
                ..default()
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: SCOREBOARD_TEXT_PADDING,
            left: SCOREBOARD_TEXT_PADDING,
            ..default()
        }),
    );

    // Walls
    commands.spawn(WallBundle::new(WallLocation::Left));
    commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Bottom));
    commands.spawn(WallBundle::new(WallLocation::Top));

    for _ in 0..10 {
        let balloon_position = Vec2::new(
            200.0 + (rng.next_u32() % 200) as f32,
            0.0 + (rng.next_u32() % 200) as f32,
        );

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::new(1.0, 1.0)),
                    ..Default::default()
                },
                texture: asset_server.load("textures/balloon.png"),
                transform: Transform {
                    translation: balloon_position.extend(0.0),
                    scale: Vec3::new(32.0, 32.0, 1.0),
                    ..default()
                },
                ..default()
            },
            Balloon,
            Collider,
        ));
    }
}

fn spritemap_fix(
    mut ev_asset: EventReader<AssetEvent<Image>>,
    mut assets: ResMut<Assets<Image>>,
) {
    for ev in ev_asset.iter() {
        if let AssetEvent::Created { handle } = ev {
            if let Some(texture) = assets.get_mut(handle) {
                texture.sampler_descriptor = ImageSampler::nearest()
            }
        }
    }
}

fn handle_mouse(
    mut commands: Commands,
    mouse_input: Res<Input<MouseButton>>,
    query: Query<&Transform, With<Monkey>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    asset_server: Res<AssetServer>,
) {
    if mouse_input.just_released(MouseButton::Left) {
        if let Some(mouse_pos) = q_windows.single().cursor_position() {
            let (camera, camera_transform) = q_camera.single();
            if let Some(mouse_pos) = camera
                .viewport_to_world(camera_transform, mouse_pos)
                .map(|ray| ray.origin.truncate())
            {
                let monkey_pos = query.get_single().unwrap().translation;

                let dir = monkey_pos.truncate() - mouse_pos;
                let speed = dir.length();

                commands.spawn((
                    SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(1.0, 1.0)),
                            ..Default::default()
                        },
                        texture: asset_server.load("textures/arrow.png"),
                        transform: Transform::from_translation(mouse_pos.extend(0.0))
                            .with_scale(Vec3::new(32.0, 32.0, 0.0)),
                        ..default()
                    },
                    Arrow,
                    Velocity(dir.normalize() * speed.min(100.0) * 10.0),
                    Falling,
                ));
            }
        }
    }
}

fn rotate_arrows(mut query: Query<(&mut Transform, &Velocity), With<Arrow>>) {
    for (mut arrow_transform, arrow_velocity) in &mut query {
        let angle = arrow_velocity.0.y.atan2(arrow_velocity.0.x) - PI / 4.0;
        *arrow_transform = arrow_transform.with_rotation(Quat::from_axis_angle(Vec3::Z, angle));
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time_step: Res<FixedTime>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time_step.period.as_secs_f32();
        transform.translation.y += velocity.y * time_step.period.as_secs_f32();
    }
}

fn apply_gravity(mut query: Query<&mut Velocity, With<Falling>>, time_step: Res<FixedTime>) {
    for mut velocity in &mut query {
        velocity.y -= GRAVITY * time_step.period.as_secs_f32();
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[1].value = scoreboard.score.to_string();
}

fn check_for_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    arrow_query: Query<&Transform, With<Arrow>>,
    collider_query: Query<(Entity, &Transform, Option<&Balloon>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for arrow_transform in &arrow_query {
        let arrow_size = arrow_transform.scale.truncate();

        // check collision with walls
        for (collider_entity, transform, collided_balloon) in &collider_query {
            let collision = collide(
                arrow_transform.translation,
                arrow_size,
                transform.translation,
                transform.scale.truncate(),
            );
            if collision.is_some() {
                // Sends a collision event so that other systems can react to the collision
                collision_events.send_default();

                // Bricks should be despawned and increment the scoreboard on collision
                if collided_balloon.is_some() {
                    scoreboard.score += 1;
                    commands.entity(collider_entity).despawn();
                }
            }
        }
    }
}

fn play_collision_sound(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    sound: Res<CollisionSound>,
) {
    // Play a sound once per frame if a collision occurred.
    if !collision_events.is_empty() {
        // This prevents events staying active on the next frame.
        collision_events.clear();
        commands.spawn(AudioBundle {
            source: sound.0.clone(),
            // auto-despawn the entity when playback finishes
            settings: PlaybackSettings::DESPAWN,
        });
    }
}
