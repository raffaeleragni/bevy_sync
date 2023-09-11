use bevy::render::mesh::Indices;
use bevy::render::mesh::VertexAttributeValues::{Float32x2, Float32x3, Float32x4};
use bevy::{prelude::*, render::render_resource::PrimitiveTopology};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MeshData(
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<[f32; 4]>,
    Vec<u32>,
);

pub(crate) fn mesh_to_bin(mesh: &Mesh) -> Vec<u8> {
    if let Some(Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        if let Some(Float32x3(normals)) = mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
            if let Some(Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                if let Some(Float32x4(tangents)) = mesh.attribute(Mesh::ATTRIBUTE_TANGENT) {
                    if let Some(Indices::U32(indices)) = mesh.indices() {
                        let data = MeshData(
                            positions.clone(),
                            normals.clone(),
                            uvs.clone(),
                            tangents.clone(),
                            indices.clone(),
                        );
                        let binary = bincode::serialize(&data).unwrap();
                        return binary;
                    }
                }
            }
        }
    }
    vec![]
}

pub(crate) fn bin_to_mesh(binary: &[u8]) -> Mesh {
    let data: MeshData = bincode::deserialize(binary).unwrap();
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Float32x3(data.0));
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Float32x3(data.1));
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Float32x2(data.2));
    mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, Float32x4(data.3));
    mesh.set_indices(Some(Indices::U32(data.4)));
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
}
