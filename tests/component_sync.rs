mod assert;
mod setup;

use bevy::{
    pbr::{CascadeShadowConfig, Cascades, CascadesVisibleEntities, CubemapVisibleEntities},
    prelude::*,
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::{CascadesFrusta, CubemapFrusta, Frustum},
    },
};
use bevy_sync::{SyncEntity, SyncExclude, SyncMark};
use serial_test::serial;
use setup::{MyNonSynched, MySynched, TestEnv, TestRun};
use uuid::Uuid;

use crate::assert::{
    count_entities_with_component, count_entities_without_component, find_entity_with_server_id,
    get_first_entity_component,
};

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.server.world_mut().spawn((SyncMark {}, MyNonSynched {}));
            0
        },
        |env, _, _| {
            let count = count_entities_with_component::<MyNonSynched>(&mut env.clients[0]);
            assert_eq!(count, 0);
        },
    );
}

#[test]
#[serial]
fn test_non_marked_component_is_not_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.clients[0]
                .world_mut()
                .spawn((SyncMark {}, MyNonSynched {}))
                .id()
        },
        |env, _, _| {
            let count = count_entities_without_component::<MyNonSynched>(&mut env.clients[0]);
            assert_eq!(count, 0);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(MySynched { value: 7 });
        },
        |env, _, _| {
            let world = env.clients[0].world_mut();
            let comp = get_first_entity_component::<MySynched>(world).unwrap();
            assert_eq!(comp.value, 7);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_server_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(10);

            env.server
                .world_mut()
                .entity_mut(e_id)
                .insert(MySynched { value: 7 });
            env.update(10);

            env.server
                .world_mut()
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(10);

            0
        },
        |env, _, _| {
            let world = env.clients[0].world_mut();
            let comp = get_first_entity_component::<MySynched>(world).unwrap();
            assert_eq!(comp.value, 3);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.clients[0].world_mut().entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            let server_e_id = e.get::<SyncEntity>().unwrap().uuid;
            server_e_id
        },
        |env, _, id: Uuid| {
            let e = find_entity_with_server_id(&mut env.server, id).unwrap();
            let e = env.server.world_mut().entity(e);
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 7);
        },
    );
}

#[test]
#[serial]
fn test_marked_component_is_transferred_from_client_then_changed() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env: &mut TestEnv| {
            env.setup_registration::<MySynched>();
            let e_id = env.clients[0].world_mut().spawn(SyncMark {}).id();
            env.update(3);

            let mut e = env.clients[0].world_mut().entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            env.update(3);

            env.clients[0]
                .world_mut()
                .entity_mut(e_id)
                .get_mut::<MySynched>()
                .unwrap()
                .value = 3;
            env.update(3);

            let server_e_id = env.clients[0]
                .world_mut()
                .entity_mut(e_id)
                .get::<SyncEntity>()
                .unwrap()
                .uuid;
            server_e_id
        },
        |env: &mut TestEnv, _, id: Uuid| {
            let e = find_entity_with_server_id(&mut env.server, id).unwrap();
            let e = env.server.world_mut().entity(e);
            let compo = e.get::<MySynched>().unwrap();
            assert_eq!(compo.value, 3);
        },
    );
}

#[test]
#[serial]
fn exclusion_marked_will_not_be_synced() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<MySynched>();
            let e_id = env
                .server
                .world_mut()
                .spawn((SyncMark {}, SyncExclude::<MySynched>::default()))
                .id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(MySynched { value: 7 });
            0
        },
        |env, _, _| {
            let mut count_check = 0;
            for _ in env.clients[0]
                .world_mut()
                .query_filtered::<Entity, With<MySynched>>()
                .iter(env.clients[0].world())
            {
                count_check += 1;
            }
            assert_eq!(count_check, 0);
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_global_transform() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<Transform>();
            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(Transform::from_xyz(1.0, 2.0, 3.0));
        },
        |env, _, _| {
            let world = env.clients[0].world_mut();
            let comp = get_first_entity_component::<Transform>(world).unwrap();
            assert_eq!(comp.translation.x, 1.0);
            assert_eq!(comp.translation.y, 2.0);
            assert_eq!(comp.translation.z, 3.0);

            let _ = get_first_entity_component::<GlobalTransform>(world).unwrap();
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_computed_visibility() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<Visibility>();
            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(Visibility::default());
        },
        |env, _, _| {
            let world = env.clients[0].world_mut();
            let _ = get_first_entity_component::<Visibility>(world).unwrap();
            let _ = get_first_entity_component::<ViewVisibility>(world).unwrap();
        },
    );
}

#[test]
#[serial]
fn test_auto_spawn_for_point_light() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        |env| {
            env.setup_registration::<PointLight>();
            env.setup_registration::<SpotLight>();
            env.setup_registration::<DirectionalLight>();
            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(PointLight::default());

            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(SpotLight::default());

            let e_id = env.server.world_mut().spawn(SyncMark {}).id();
            env.update(4);
            let mut e = env.server.world_mut().entity_mut(e_id);
            e.insert(DirectionalLight::default());
        },
        |env, _, _| {
            let world = env.clients[0].world_mut();
            let _ = get_first_entity_component::<PointLight>(world).unwrap();
            let _ = get_first_entity_component::<SpotLight>(world).unwrap();
            let _ = get_first_entity_component::<DirectionalLight>(world).unwrap();
            let _ = get_first_entity_component::<CubemapFrusta>(world).unwrap();
            let _ = get_first_entity_component::<CubemapVisibleEntities>(world).unwrap();
            let _ = get_first_entity_component::<Frustum>(world).unwrap();
            let _ = get_first_entity_component::<CascadesFrusta>(world).unwrap();
            let _ = get_first_entity_component::<CascadesVisibleEntities>(world).unwrap();
            let _ = get_first_entity_component::<Cascades>(world).unwrap();
            let _ = get_first_entity_component::<CascadeShadowConfig>(world).unwrap();
        },
    );
}

#[test]
fn test_skinned_mesh_component() {
    TestRun::default().run(
        1,
        TestRun::no_pre_setup,
        setup_skinned_test,
        |env, _, test_mat| {
            assert_skinned_test(env, test_mat);
        },
    );
}

#[test]
fn test_skinned_mesh_component_from_initial() {
    TestRun::default().run(
        1,
        setup_skinned_test,
        TestRun::no_setup,
        |env, test_mat, _| assert_skinned_test(env, test_mat),
    );
}

fn setup_skinned_test(env: &mut TestEnv) -> Vec<Mat4> {
    let test_mat = vec![Mat4::from_cols(
        Vec4::new(1.0, 1.1, 1.2, 1.4),
        Vec4::new(2.0, 2.1, 2.2, 2.4),
        Vec4::new(3.0, 3.1, 3.2, 3.4),
        Vec4::new(4.0, 4.1, 4.2, 4.4),
    )];

    env.setup_registration::<SkinnedMesh>();
    env.setup_registration::<Name>();
    let world = &mut env.server.world_mut();
    let mut joints = Vec::<Entity>::new();
    for i in 0..4 {
        joints.push(world.spawn((SyncMark {}, Name::new(format!("{i}")))).id());
    }
    env.update(10);

    let e_id = env.server.world_mut().spawn(SyncMark {}).id();
    env.update(4);
    env.server.world_mut().resource_scope(
        |world, mut assets: Mut<Assets<SkinnedMeshInverseBindposes>>| {
            let handle = assets.add(test_mat.clone());
            let mut e = world.entity_mut(e_id);
            e.insert(SkinnedMesh {
                inverse_bindposes: handle,
                joints,
            });
        },
    );
    test_mat
}

fn assert_skinned_test(env: &mut TestEnv, test_mat: Vec<Mat4>) {
    env.clients[0].world_mut().resource_scope(
        |world, assets: Mut<Assets<SkinnedMeshInverseBindposes>>| {
            let compo = get_first_entity_component::<SkinnedMesh>(world).unwrap();
            let poses = assets.get(compo.inverse_bindposes.id()).unwrap();
            let poses: Vec<Mat4> = (**poses).to_vec();
            assert_eq!(poses, test_mat);
            assert_eq!(compo.joints.len(), 4);
            let joints = compo.joints.clone();
            for (i, e) in joints.into_iter().enumerate() {
                let joint_entity_on_client = world.get_entity(e);
                assert!(joint_entity_on_client.is_ok());
                let entity = joint_entity_on_client.unwrap();
                let name = entity.get::<Name>().unwrap();
                assert_eq!(name.as_str(), format!("{i}"));
            }
        },
    );
}
