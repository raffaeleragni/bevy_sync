use bevy::render::mesh::Indices;
use bevy::render::mesh::VertexAttributeValues::{Float32x2, Float32x3, Float32x4, Uint16x4};
use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AssetImage;

#[derive(Serialize, Deserialize)]
struct MeshData {
    mesh_type: u8,
    positions: Option<Vec<[f32; 3]>>,
    normals: Option<Vec<[f32; 3]>>,
    uvs0: Option<Vec<[f32; 2]>>,
    uvs1: Option<Vec<[f32; 2]>>,
    tangents: Option<Vec<[f32; 4]>>,
    colors: Option<Vec<[f32; 4]>>,
    joint_weights: Option<Vec<[f32; 4]>>,
    joint_indices: Option<Vec<[u16; 4]>>,
    indices32: Option<Vec<u32>>,
    indices16: Option<Vec<u16>>,
    morph_targets: Option<AssetImage>, //TODO serialize Image type
    morph_target_names: Option<Vec<String>>,
}

pub(crate) fn mesh_to_bin(mesh: &Mesh) -> Vec<u8> {
    let positions = if let Some(Float32x3(t)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(t.clone())
    } else {
        None
    };

    let tangents = if let Some(Float32x4(t)) = mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
        Some(t.clone())
    } else {
        None
    };

    let normals = if let Some(Float32x3(t)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
        Some(t.clone())
    } else {
        None
    };

    let uvs0 = if let Some(Float32x2(t)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(t.clone())
    } else {
        None
    };

    let uvs1 = if let Some(Float32x2(t)) = mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
        Some(t.clone())
    } else {
        None
    };

    let colors = if let Some(Float32x4(t)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
        Some(t.clone())
    } else {
        None
    };

    let joint_weights = if let Some(Float32x4(t)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT) {
        Some(t.clone())
    } else {
        None
    };

    let joint_indices = if let Some(Uint16x4(t)) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX) {
        Some(t.clone())
    } else {
        None
    };

    let indices32 = if let Some(Indices::U32(t)) = mesh.indices() {
        Some(t.clone())
    } else {
        None
    };

    let indices16 = if let Some(Indices::U16(t)) = mesh.indices() {
        Some(t.clone())
    } else {
        None
    };

    let morph_targets = None;
    let morph_target_names = mesh.morph_target_names().map(|v| v.to_vec());
    let mesh_type_num = match mesh.primitive_topology() {
        PrimitiveTopology::PointList => 0,
        PrimitiveTopology::LineList => 1,
        PrimitiveTopology::LineStrip => 2,
        PrimitiveTopology::TriangleList => 3,
        PrimitiveTopology::TriangleStrip => 4,
    };

    let data = MeshData {
        mesh_type: mesh_type_num,
        positions,
        normals,
        uvs0,
        uvs1,
        tangents,
        colors,
        joint_weights,
        joint_indices,
        indices32,
        indices16,
        morph_targets,
        morph_target_names,
    };

    bincode::serialize(&data).unwrap()
}

pub(crate) fn bin_to_mesh(binary: &[u8]) -> Mesh {
    let Ok(data) = bincode::deserialize::<MeshData>(binary) else {
        return Mesh::new(PrimitiveTopology::TriangleList);
    };

    let mesh_type_enum = match data.mesh_type {
        0 => PrimitiveTopology::PointList,
        1 => PrimitiveTopology::LineList,
        2 => PrimitiveTopology::LineStrip,
        3 => PrimitiveTopology::TriangleList,
        4 => PrimitiveTopology::TriangleStrip,
        _ => PrimitiveTopology::TriangleList,
    };

    let mut mesh = Mesh::new(mesh_type_enum);

    if let Some(positions) = data.positions {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    }

    if let Some(normals) = data.normals {
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    }

    if let Some(uvs0) = data.uvs0 {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs0);
    }

    if let Some(uvs1) = data.uvs1 {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uvs1);
    }

    if let Some(tangents) = data.tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }

    if let Some(colors) = data.colors {
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    }

    if let Some(joint_weights) = data.joint_weights {
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights);
    }

    if let Some(joint_indices) = data.joint_indices {
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, Uint16x4(joint_indices));
    }

    if let Some(indices) = data.indices32 {
        mesh.set_indices(Some(Indices::U32(indices)));
    }
    if let Some(indices) = data.indices16 {
        mesh.set_indices(Some(Indices::U16(indices)));
    }

    //if let Some(morph_targets) = data.morph_targets {
    //    mesh.set_morph_targets(morph_targets);
    //}

    if let Some(morph_target_names) = data.morph_target_names {
        mesh.set_morph_target_names(morph_target_names);
    }

    mesh
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mesh_to_bin_to_mesh_compare() {
        let mesh = sample_mesh();

        let binary = mesh_to_bin(&mesh);
        let mesh2 = bin_to_mesh(&binary[..]);

        assert_eq!(mesh.primitive_topology(), mesh2.primitive_topology());
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT).unwrap().get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_TANGENT)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                .unwrap()
                .get_bytes()
        );
        let Indices::U32(v1) = mesh.indices().unwrap() else {
            panic!("bad indices type")
        };
        let Indices::U32(v2) = mesh2.indices().unwrap() else {
            panic!("bad indices type")
        };
        assert_eq!(mesh.morph_target_names(), mesh2.morph_target_names());
        assert_eq!(v1, v2);
    }

    #[test]
    fn mesh_to_bin_to_mesh_idx16_compare() {
        let mesh = sample_mesh_idx16();

        let binary = mesh_to_bin(&mesh);
        let mesh2 = bin_to_mesh(&binary[..]);

        assert_eq!(mesh.primitive_topology(), mesh2.primitive_topology());
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT).unwrap().get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_TANGENT)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
                .unwrap()
                .get_bytes()
        );
        let Indices::U16(v1) = mesh.indices().unwrap() else {
            panic!("bad indices type")
        };
        let Indices::U16(v2) = mesh2.indices().unwrap() else {
            panic!("bad indices type")
        };
        assert_eq!(mesh.morph_target_names(), mesh2.morph_target_names());
        assert_eq!(v1, v2);
    }

    #[test]
    fn mesh_to_bin_to_mesh_compare_no_tangents() {
        let mesh = sample_mesh_no_tangents();

        let binary = mesh_to_bin(&mesh);
        let mesh2 = bin_to_mesh(&binary[..]);

        assert_eq!(mesh.primitive_topology(), mesh2.primitive_topology());
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_POSITION)
                .unwrap()
                .get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_NORMAL).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().get_bytes()
        );
        assert_eq!(
            mesh.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes(),
            mesh2.attribute(Mesh::ATTRIBUTE_UV_1).unwrap().get_bytes()
        );
        assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_none());
        assert!(mesh2.attribute(Mesh::ATTRIBUTE_TANGENT).is_none());
        let Indices::U32(v1) = mesh.indices().unwrap() else {
            panic!("bad indices type")
        };
        let Indices::U32(v2) = mesh2.indices().unwrap() else {
            panic!("bad indices type")
        };
        assert_eq!(v1, v2);
    }

    fn sample_mesh() -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 2., 1.], [2., 0., 0.]],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[1., 1.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0., 1., 0., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0., 1., 0., 0.]; 4]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0., 1., 0., 0.]; 4]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            Uint16x4(vec![[1u16, 2, 3, 4]; 4]),
        );
        mesh.set_indices(Some(Indices::U32(vec![0, 2, 1])));
        mesh.set_morph_target_names(vec!["name1".into(), "name2".into()]);
        mesh
    }

    fn sample_mesh_idx16() -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 2., 1.], [2., 0., 0.]],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[1., 1.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0., 1., 0., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0., 1., 0., 0.]; 4]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0., 1., 0., 0.]; 4]);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            Uint16x4(vec![[1u16, 2, 3, 4]; 4]),
        );
        mesh.set_indices(Some(Indices::U16(vec![0, 2, 1])));
        mesh.set_morph_target_names(vec!["name1".into(), "name2".into()]);
        mesh
    }

    fn sample_mesh_no_tangents() -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[0., 0., 0.], [1., 2., 1.], [2., 0., 0.]],
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0., 0.]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, vec![[1., 1.]; 3]);
        mesh.set_indices(Some(Indices::U32(vec![0, 2, 1])));
        mesh
    }
}
