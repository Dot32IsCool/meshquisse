use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::RapierContext;
use bevy_transform_gizmo::TransformGizmoSystem;

use crate::{
    screen_physics_ray_cast,
    tools::{self, TriangleMesh},
    MainCamera,
};

pub struct InteractMeshPlugin;

impl Plugin for InteractMeshPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(bevy_mod_picking::DefaultPickingPlugins)
            .add_plugin(bevy_transform_gizmo::TransformGizmoPlugin::default())
            .add_startup_system(init_assets)
            .add_stage_before(
                CoreStage::PreUpdate,
                "before_preupdate",
                SystemStage::parallel(),
            )
            .add_system_to_stage("before_preupdate", adapt_camera)
            .add_system(spawn_vertices_selectable)
            .add_system(update_vertices_position)
            .add_system(spawn_visual_mesh)
            .add_system(update_visual_mesh);
    }
}

/// Meant to be used in correlation with `ShowAndUpdateMesh` and/or `EditableMesh`
#[derive(Component)]
pub struct TriangleMeshData(pub TriangleMesh);

/// will spawn a bevy mesh, and update its visual if its `TriangleMeshData` changes.
#[derive(Component, Default)]
pub struct ShowAndUpdateMesh(pub Option<Handle<Mesh>>);

/// will spawn children selectable handles via bevy_transform_gizmo.
/// When these gizmos are updated, they reach for their parent `EditableMesh`
/// and update its mesh.
#[derive(Component)]
pub struct EditableMesh;

#[derive(Component)]
pub struct EditableMeshVertex {
    pub vertex_id: u32,
}

pub struct InteractAssets {
    gizmo_mesh: Handle<Mesh>,
    gizmo_mesh_mat: Handle<StandardMaterial>,
    visual_mesh_mat: Handle<StandardMaterial>,
}

fn init_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(InteractAssets {
        gizmo_mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        gizmo_mesh_mat: materials.add(Color::rgb(0.99, 0.2, 0.3).into()),
        visual_mesh_mat: materials.add(Color::rgb(0.3, 0.99, 0.2).into()),
    });
}
fn adapt_camera(mut commands: Commands, q_cam: Query<Entity, Added<MainCamera>>) {
    for e in q_cam.iter() {
        commands
            .entity(e)
            .insert_bundle(bevy_mod_picking::PickingCameraBundle::default())
            .insert(bevy_transform_gizmo::GizmoPickSource::default());
    }
}

fn spawn_visual_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<InteractAssets>,
    mut q_new_shown_meshes: Query<
        (Entity, &TriangleMeshData, &mut ShowAndUpdateMesh),
        Added<ShowAndUpdateMesh>,
    >,
) {
    for (e, mesh_data, mut show_update_mesh) in q_new_shown_meshes.iter_mut() {
        let mesh_handle = meshes.add(tools::bevymesh_from_trimesh(&mesh_data.0));
        (*show_update_mesh).0 = Some(dbg!(mesh_handle.clone()));
        commands.entity(e).insert_bundle(PbrBundle {
            mesh: mesh_handle,
            material: assets.visual_mesh_mat.clone(),
            ..default()
        });
    }
}
fn update_visual_mesh(
    mut meshes: ResMut<Assets<Mesh>>,
    q_updated_meshes: Query<(&ShowAndUpdateMesh, &TriangleMeshData), Changed<TriangleMeshData>>,
) {
    for (update, mesh_data) in q_updated_meshes.iter() {
        if let Some(mesh_handle) = update.0.as_ref() {
            if let Some(mesh) = meshes.get_mut(mesh_handle) {
                if let Some(bevy::render::mesh::VertexAttributeValues::Float32x3(
                    ref mut positions,
                )) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    positions
                        .iter_mut()
                        .enumerate()
                        .for_each(|(index, position)| {
                            let pos_data = mesh_data.0.positions[index];
                            position[0] = pos_data.x;
                            position[1] = 0f32;
                            position[2] = pos_data.y;
                        });
                }
            }
        }
    }
}

fn spawn_vertices_selectable(
    mut commands: Commands,
    assets: Res<InteractAssets>,
    q_new_editable_meshes: Query<(Entity, &TriangleMeshData), Added<EditableMesh>>,
) {
    for (e, mesh_data) in q_new_editable_meshes.iter() {
        commands.entity(e).add_children(|parent| {
            for (vertex_id, position) in mesh_data.0.positions.iter().enumerate() {
                parent
                    .spawn_bundle(PbrBundle {
                        mesh: assets.gizmo_mesh.clone(),
                        material: assets.gizmo_mesh_mat.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            position.x, 0f32, position.y,
                        )),
                        ..Default::default()
                    })
                    .insert(EditableMeshVertex {
                        vertex_id: vertex_id as u32,
                    })
                    .insert_bundle(bevy_mod_picking::PickableBundle::default())
                    .insert(bevy_transform_gizmo::GizmoTransformable);
            }
        });
    }
}

fn update_vertices_position(
    q_changed_vertices: Query<(&Parent, &EditableMeshVertex, &Transform), Changed<Transform>>,
    mut q_parent_mesh_data: Query<&mut TriangleMeshData>,
) {
    for (parent, vertex, transform) in q_changed_vertices.iter() {
        if let Ok(mut mesh_to_edit) = q_parent_mesh_data.get_mut(parent.get()) {
            mesh_to_edit.0.positions[vertex.vertex_id as usize] = transform.translation.xz();
        }
    }
}
