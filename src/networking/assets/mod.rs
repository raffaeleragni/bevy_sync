mod mesh_serde;

use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        mpsc::{channel, Receiver},
        Arc, RwLock,
    },
    time::Duration,
};

use crate::{lib_priv::SyncTrackerRes, proto::SyncAssetType};
use bevy::utils::Uuid;
use bevy::{prelude::*, utils::HashMap};
use mesh_serde::{bin_to_mesh, mesh_to_bin};
use std::io::Read;
use threadpool::ThreadPool;
use tiny_http::{Request, Response, Server};

pub(crate) fn init(app: &mut App, addr: IpAddr, port: u16) {
    debug!(
        "Initializing asset sync on {:?}:{}",
        addr.clone(),
        port.clone()
    );
    let http_transfer = SyncAssetTransfer::new(addr, port);
    app.insert_resource(http_transfer);
    app.add_systems(
        Update,
        process_mesh_assets.run_if(resource_exists::<SyncAssetTransfer>()),
    );
}

fn process_mesh_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    sync: ResMut<SyncAssetTransfer>,
    mut sync_tracker: ResMut<SyncTrackerRes>,
) {
    let Ok(mut map) = sync.meshes_to_apply.write() else {
        return;
    };
    for (id, mesh) in map.drain() {
        sync_tracker.pushed_handles_from_network.insert(id);
        let id: AssetId<Mesh> = AssetId::Uuid { uuid: id };
        meshes.insert(id, mesh);
    }
}

type MeshCache = Arc<RwLock<HashMap<Uuid, Mesh>>>;

#[derive(Resource)]
pub(crate) struct SyncAssetTransfer {
    base_url: String,
    server_pool: ThreadPool,
    download_pool: ThreadPool,
    meshes: MeshCache,
    meshes_to_apply: MeshCache,
}

impl SyncAssetTransfer {
    pub(crate) fn new(addr: IpAddr, port: u16) -> Self {
        let bind = SocketAddr::new(addr, port);
        let server_pool = ThreadPool::new(2);
        let download_pool = ThreadPool::new(127);
        let meshes = Arc::new(RwLock::new(HashMap::<Uuid, Mesh>::new()));
        let meshes_to_apply = Arc::new(RwLock::new(HashMap::<Uuid, Mesh>::new()));

        let (server_tx, server_rx) = channel::<Request>();
        let server = Server::http(bind).unwrap();
        let result = Self {
            base_url: format!("http://{}:{}", addr, port),
            server_pool,
            download_pool,
            meshes,
            meshes_to_apply,
        };
        result.server_pool.execute(move || {
            for request in server.incoming_requests() {
                debug!("Queuing response to {}", request.url());
                server_tx.send(request).unwrap_or(());
            }
        });
        let meshes = result.meshes.clone();
        result
            .server_pool
            .execute(|| Self::respond(server_rx, meshes));
        result
    }

    pub(crate) fn request(&self, asset_type: SyncAssetType, id: Uuid, url: String) {
        if let Ok(meshes) = self.meshes.read() {
            if meshes.contains_key(&id) {
                return;
            }
        }
        let meshes_to_apply = self.meshes_to_apply.clone();
        debug!("Queuing request for {:?}:{} at {}", asset_type, id, url);
        self.download_pool.execute(move || {
            if let Ok(response) = ureq::get(url.as_str()).call() {
                let len = response
                    .header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap();
                let mut bytes: Vec<u8> = Vec::with_capacity(len);
                if response
                    .into_reader()
                    .take(100_000_000)
                    .read_to_end(&mut bytes)
                    .is_ok()
                {
                    match asset_type {
                        SyncAssetType::Mesh => {
                            let mesh = bin_to_mesh(bytes.as_slice());
                            let mut lock = meshes_to_apply.write();
                            loop {
                                match lock {
                                    Ok(mut map) => {
                                        debug!("Received mesh {}", id);
                                        map.insert(id, mesh);
                                        break;
                                    }
                                    Err(_) => lock = meshes_to_apply.write(),
                                }
                                std::thread::sleep(Duration::from_millis(1));
                            }
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn serve(&mut self, _: SyncAssetType, id: &Uuid, mesh: &Mesh) -> String {
        let mut lock = self.meshes.write();
        loop {
            match lock {
                Ok(mut map) => {
                    debug!("Servig mesh {}", id);
                    map.insert(*id, mesh.clone());
                    break;
                }
                Err(_) => lock = self.meshes.write(),
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        format!("{}/mesh/{}", self.base_url, &id.to_string())
    }

    fn respond(rx: Receiver<Request>, meshes: MeshCache) {
        for request in rx.iter() {
            let url = request.url();
            let Some(id) = url.strip_prefix("/mesh/") else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(id) else {
                continue;
            };

            debug!("Responding to {}", url);
            let Ok(meshesmap) = meshes.read() else {
                request
                    .respond(Response::from_string("").with_status_code(449))
                    .unwrap_or(());
                continue;
            };
            let Some(mesh) = meshesmap.get(&id) else {
                request
                    .respond(Response::from_string("").with_status_code(404))
                    .unwrap_or(());
                continue;
            };
            request
                .respond(Response::from_data(mesh_to_bin(mesh)))
                .unwrap_or(());
        }
    }
}
