use bevy::{pbr::CubemapVisibleEntities, prelude::*, render::primitives::CubemapFrusta};

/// Fixing some bundle situations that are not known at sync time because bundles disappear once applied

pub(crate) struct BundleFixPlugin;

impl Plugin for BundleFixPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Update,
            (
                fix_computed_visibility,
                fix_missing_global_transforms,
                fix_missing_cubemap_frusta,
                fix_missing_cubemap_visible_entities,
            ),
        );
    }
}

fn fix_computed_visibility(
    mut cmd: Commands,
    query: Query<Entity, (With<Visibility>, Without<ViewVisibility>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(ViewVisibility::default());
    }
}

fn fix_missing_global_transforms(
    mut cmd: Commands,
    query: Query<(Entity, &Transform), Without<GlobalTransform>>,
) {
    for (e, &t) in query.iter() {
        cmd.entity(e).insert(GlobalTransform::from(t));
    }
}

fn fix_missing_cubemap_frusta(
    mut cmd: Commands,
    query: Query<Entity, (With<PointLight>, Without<CubemapFrusta>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CubemapFrusta::default());
    }
}

fn fix_missing_cubemap_visible_entities(
    mut cmd: Commands,
    query: Query<Entity, (With<PointLight>, Without<CubemapVisibleEntities>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CubemapVisibleEntities::default());
    }
}
