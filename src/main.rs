use std::{f32::consts::PI, hash::Hash, mem::transmute};

use bevy::{
    asset::AssetServerSettings, prelude::*, render::camera::RenderTarget,
    sprite::MaterialMesh2dBundle,
};
use bevy_common_assets::yaml::YamlAssetPlugin;

#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "7ea299e0-4eef-11ed-bdc3-0242ac120002"]
struct Config {
    main_ball: Ball,
    follower_ball: Ball,
    dampening_frequency: f32,
    dampening_strength: f32,
    dampening_response: f32,
}

#[derive(serde::Deserialize, bevy::reflect::TypeUuid)]
#[uuid = "e53acbf0-4eef-11ed-bdc3-0242ac120002"]
struct Ball {
    size: f32,
    starting_position: Vec3,
    color: Color,
}

#[derive(Component)]
struct MainBall;

#[derive(Component)]
struct FollowerBall;

#[derive(Component)]
struct MainCamera;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum AppState {
    Loading,
    Level,
}

#[derive(Clone, PartialEq, Debug)]
struct MainBallVelocityState {
    velocity: (f32, f32),
}

impl Default for MainBallVelocityState{
    fn default() -> Self {
        Self { velocity: (0.0,0.0) }
    }
}

impl Eq for MainBallVelocityState {}

impl Hash for MainBallVelocityState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let a: &(u32, u32) = unsafe { transmute(&self.velocity) };
        a.hash(state);
    }
}

#[derive(Clone, PartialEq, Debug)]
struct FollowerBallVelocityState {
    velocity: (f32, f32),
}

impl Default for FollowerBallVelocityState{
    fn default() -> Self {
        Self { velocity: (0.0,0.0) }
    }
}

impl Eq for FollowerBallVelocityState {}

impl Hash for FollowerBallVelocityState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let a: &(u32, u32) = unsafe { transmute(&self.velocity) };
        a.hash(state);
    }
}

fn main() {
    App::new()
        .insert_resource(AssetServerSettings {
            watch_for_changes: true,
            asset_folder: "assets".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(YamlAssetPlugin::<Config>::new(&["config.yaml"]))
        .insert_resource(Msaa { samples: 1 })
        .add_state(AppState::Loading)
        .add_state(MainBallVelocityState::default())
        .add_state(FollowerBallVelocityState::default())
        .add_startup_system(setup)
        .add_system_set(SystemSet::on_update(AppState::Loading).with_system(spawn_level))
        .add_system(main_ball_movement)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let config: Handle<Config> = asset_server.load("main.config.yaml");
    commands.insert_resource(config);
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert(MainCamera);
}

fn spawn_level(
    mut commands: Commands,
    config: Res<Handle<Config>>,
    configs: Res<Assets<Config>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut state: ResMut<State<AppState>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Some(config) = configs.get(&config) {
        commands
            .spawn_bundle(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Circle::new(config.main_ball.size).into())
                    .into(),
                material: materials.add(ColorMaterial::from(config.main_ball.color)),
                transform: Transform::from_translation(config.main_ball.starting_position),
                ..default()
            })
            .insert(MainBall);
        commands
            .spawn_bundle(MaterialMesh2dBundle {
                mesh: meshes
                    .add(shape::Circle::new(config.follower_ball.size).into())
                    .into(),
                material: materials.add(ColorMaterial::from(config.follower_ball.color)),
                transform: Transform::from_translation(config.follower_ball.starting_position),
                ..default()
            })
            .insert(FollowerBall);
        state.set(AppState::Level).unwrap()
    }
}

fn main_ball_movement(
    mut query: Query<&mut Transform, With<MainBall>>,
    wnds: Res<Windows>,
    time: Res<Time>,
    mut velocity: ResMut<State<MainBallVelocityState>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let (camera, camera_transform) = q_camera.single();
    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };
    if let Some(screen_pos) = wnd.cursor_position() {
        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);
        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;
        let ndc_to_world = camera_transform.compute_matrix() * camera.projection_matrix().inverse();
        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));
        let world_pos: Vec2 = world_pos.truncate();
        for mut transform in &mut query {
            velocity
                .set(MainBallVelocityState {
                    velocity: (
                        (world_pos.x - transform.translation.x) / time.delta_seconds(),
                        (world_pos.y - transform.translation.y) / time.delta_seconds(),
                    ),
                })
                .unwrap();
            transform.translation = Vec3::new(world_pos.x, world_pos.y, 0.0);
        }
    }
}

fn follower_ball_movement(
    main_ball: Query<&Transform, With<MainBall>>,
    mut follower_ball: Query<&mut Transform, With<FollowerBall>>,
    main_ball_velocity: Res<State<MainBallVelocityState>>,
    follower_ball_velocity: Res<State<FollowerBallVelocityState>>,
    config: Res<Handle<Config>>,
    configs: Res<Assets<Config>>,
    time: Res<Time>,
) {
    if let Some(config) = configs.get(&config) {
        let k1 = config.dampening_strength / (PI * config.dampening_frequency);
        let k2 = 1.0 / (2.0 * PI * config.dampening_frequency);
        let k3 = config.dampening_response * config.dampening_strength
            / (2.0 * PI * config.dampening_frequency);

        for mut follower_transform in &mut follower_ball {
            for main_transform in main_ball.iter() {
                follower_transform.translation += time.delta_seconds()*follower_ball_velocity.as_ref().current().velocity

            }
        }
    }


}
