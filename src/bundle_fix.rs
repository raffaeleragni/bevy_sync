use bevy::{
    pbr::{CascadeShadowConfig, Cascades, CascadesVisibleEntities, CubemapVisibleEntities},
    prelude::*,
    render::primitives::{CascadesFrusta, CubemapFrusta, Frustum},
};

/// Fixing some bundle situations that are not known at sync time because bundles disappear once applied

pub(crate) struct BundleFixPlugin;

impl Plugin for BundleFixPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Update,
            (
                fix_visibility_bundle,
                fix_missing_global_transforms,
                fix_missing_cubemap_frusta,
                fix_missing_cubemap_visible_entities,
                fix_missing_cubemap_frustum_spot,
                fix_missing_cubemap_frusta_directional,
                fix_missing_cubemap_visible_entities_directional,
                fix_missing_cascades_directional,
                fix_missing_cascades_shadow_config_directional,
            ),
        );
    }
}

#[allow(clippy::type_complexity)]
fn fix_visibility_bundle(
    mut cmd: Commands,
    query: Query<
        (Entity, &Visibility),
        (
            Added<Visibility>,
            Without<ViewVisibility>,
            Without<InheritedVisibility>,
        ),
    >,
) {
    for (e, v) in query.iter() {
        cmd.entity(e)
            .insert(*v)
            .insert(ViewVisibility::default())
            .insert(InheritedVisibility::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_global_transforms(
    mut cmd: Commands,
    query: Query<(Entity, &Transform), (Added<Transform>, Without<GlobalTransform>)>,
) {
    for (e, &t) in query.iter() {
        cmd.entity(e).insert(t).insert(GlobalTransform::from(t));
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cubemap_frusta(
    mut cmd: Commands,
    query: Query<Entity, (Added<PointLight>, Without<CubemapFrusta>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CubemapFrusta::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cubemap_visible_entities(
    mut cmd: Commands,
    query: Query<Entity, (Added<PointLight>, Without<CubemapVisibleEntities>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CubemapVisibleEntities::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cubemap_frustum_spot(
    mut cmd: Commands,
    query: Query<Entity, (Added<SpotLight>, Without<Frustum>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(Frustum::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cubemap_frusta_directional(
    mut cmd: Commands,
    query: Query<Entity, (Added<DirectionalLight>, Without<CascadesFrusta>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CascadesFrusta::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cubemap_visible_entities_directional(
    mut cmd: Commands,
    query: Query<Entity, (Added<DirectionalLight>, Without<CascadesVisibleEntities>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CascadesVisibleEntities::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cascades_directional(
    mut cmd: Commands,
    query: Query<Entity, (Added<DirectionalLight>, Without<Cascades>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(Cascades::default());
    }
}

#[allow(clippy::type_complexity)]
fn fix_missing_cascades_shadow_config_directional(
    mut cmd: Commands,
    query: Query<Entity, (Added<DirectionalLight>, Without<CascadeShadowConfig>)>,
) {
    for e in query.iter() {
        cmd.entity(e).insert(CascadeShadowConfig::default());
    }
}
