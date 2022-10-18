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
struct MainCamera;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum AppState {
    Loading,
    Level,
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
        .add_startup_system(setup)
        .add_system_set(SystemSet::on_update(AppState::Loading).with_system(spawn_level))
        .add_system(run)
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
    mut configs: ResMut<Assets<Config>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut state: ResMut<State<AppState>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Some(config) = configs.remove(config.id) {
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
        state.set(AppState::Level).unwrap()
    }
}

fn run(
    mut query: Query<&mut Transform, With<MainBall>>,
    wnds: Res<Windows>,
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
            transform.translation = Vec3::new(world_pos.x, world_pos.y, 0.0);
        }
    }
}
