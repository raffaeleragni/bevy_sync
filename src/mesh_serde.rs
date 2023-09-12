use bevy::render::mesh::Indices;
use bevy::render::mesh::VertexAttributeValues::{Float32x2, Float32x3, Float32x4};
use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MeshData {
    mesh_type: u8,
    positions: Option<Vec<[f32; 3]>>,
    normals: Option<Vec<[f32; 3]>>,
    uvs: Option<Vec<[f32; 2]>>,
    tangents: Option<Vec<[f32; 4]>>,
    indices: Option<Vec<u32>>,
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

    let uvs = if let Some(Float32x2(t)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
        Some(t.clone())
    } else {
        None
    };

    let indices = if let Some(Indices::U32(t)) = mesh.indices() {
        Some(t.clone())
    } else {
        None
    };

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
        uvs,
        tangents,
        indices,
    };

    bincode::serialize(&data).unwrap()
}

pub(crate) fn bin_to_mesh(binary: &[u8]) -> Mesh {
    let Ok(data) = bincode::deserialize::<MeshData>(binary) else { return Mesh::new(PrimitiveTopology::TriangleList) };

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

    if let Some(uvs) = data.uvs {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    }

    if let Some(tangents) = data.tangents {
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, tangents);
    }

    if let Some(indices) = data.indices {
        mesh.set_indices(Some(Indices::U32(indices)));
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
            mesh.attribute(Mesh::ATTRIBUTE_TANGENT).unwrap().get_bytes(),
            mesh2
                .attribute(Mesh::ATTRIBUTE_TANGENT)
                .unwrap()
                .get_bytes()
        );
        let Indices::U32(v1) = mesh.indices().unwrap() else {panic!("bad indices type")};
        let Indices::U32(v2) = mesh2.indices().unwrap() else {panic!("bad indices type")};
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
        assert!(mesh.attribute(Mesh::ATTRIBUTE_TANGENT).is_none());
        assert!(mesh2.attribute(Mesh::ATTRIBUTE_TANGENT).is_none());
        let Indices::U32(v1) = mesh.indices().unwrap() else {panic!("bad indices type")};
        let Indices::U32(v2) = mesh2.indices().unwrap() else {panic!("bad indices type")};
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
        mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vec![[0., 1., 0., 0.]; 3]);
        mesh.set_indices(Some(Indices::U32(vec![0, 2, 1])));
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
        mesh.set_indices(Some(Indices::U32(vec![0, 2, 1])));
        mesh
    }
}
