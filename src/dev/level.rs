use bevy::{math::vec3, prelude::*};

const FLOOR_SIZE: Vec3 = vec3(50.0, 1.0, 50.0);
const FLOOR_COLOR: Color = Color::rgb(0.5, 0.5, 0.5);
const FLOOR_POSITION: Vec3 = vec3(0.0, -0.5, 0.0);
const LIGHT_POSITION: Vec3 = vec3(0.0, 50.0, 0.0);
const LIGHT_ROTATION_X: f32 = -std::f32::consts::PI / 4.0;
const CAMERA_POSITION: Vec3 = vec3(0.0, 75.0, 25.0);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (
            setup_light, 
            setup_fixed_camera, 
            setup_floor
        ));
    }
}

fn setup_floor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    commands.spawn(PbrBundle{
        mesh: meshes.add(Mesh::from(Cuboid::from_size(FLOOR_SIZE))),
        material: materials.add(FLOOR_COLOR),
        transform: Transform::from_translation(FLOOR_POSITION),
        ..default()
    });
}

fn setup_light(mut commands: Commands) {
    commands.spawn(DirectionalLightBundle{
        directional_light: DirectionalLight{
            shadows_enabled: true,
            ..default()
        },
        transform: Transform{
            translation: LIGHT_POSITION,
            rotation: Quat::from_rotation_x(LIGHT_ROTATION_X),
            ..default()
        },
        ..default()
    });
}

fn setup_fixed_camera(mut commands: Commands) {
    commands.spawn(Camera3dBundle{
        transform: Transform::from_translation(CAMERA_POSITION)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
