use bevy::{prelude::*, render::render_resource::PrimitiveTopology};

pub(crate) fn mesh_to_bin(_: &Mesh) -> Vec<u8> {
    vec![]
}

pub(crate) fn bin_to_mesh(_: &[u8]) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList)
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
    }

    fn sample_mesh() -> Mesh {
        Mesh::new(PrimitiveTopology::TriangleList)
    }
}
