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

pub(crate) fn init(app: &mut App, addr: IpAddr, port: u16, max_transfer: usize) {
    debug!(
        "Initializing asset sync on {:?}:{}",
        addr.clone(),
        port.clone()
    );
    let http_transfer = SyncAssetTransfer::new(addr, port, max_transfer);
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
        meshes.insert(id, bin_to_mesh(&mesh));
    }
}

type MeshCache = Arc<RwLock<HashMap<Uuid, Vec<u8>>>>;

#[derive(Resource)]
pub(crate) struct SyncAssetTransfer {
    base_url: String,
    server_pool: ThreadPool,
    download_pool: ThreadPool,
    meshes: MeshCache,
    meshes_to_apply: MeshCache,
    max_transfer: usize,
}

impl SyncAssetTransfer {
    pub(crate) fn new(addr: IpAddr, port: u16, max_transfer: usize) -> Self {
        let base_url = format!("http://{}:{}", addr, port);
        debug!("Starting asset server on {}", base_url);
        let server = Server::http(SocketAddr::new(addr, port)).unwrap();

        let server_pool = ThreadPool::new(128);
        let download_pool = ThreadPool::new(127);

        let meshes = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let meshes_to_apply = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));

        let result = Self {
            base_url,
            server_pool,
            download_pool,
            meshes,
            meshes_to_apply,
            max_transfer,
        };

        let (server_tx, server_rx) = channel::<Request>();
        result.server_pool.execute(move || {
            for request in server.incoming_requests() {
                debug!("Queuing response to {}", request.url());
                server_tx.send(request).unwrap_or(());
            }
        });
        let meshes = result.meshes.clone();
        result
            .server_pool
            .execute(move || Self::respond(server_rx, meshes, max_transfer));
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
        let max_transfer = self.max_transfer;
        self.download_pool.execute(move || {
            if let Ok(response) = ureq::get(url.as_str()).call() {
                let len = response
                    .header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(max_transfer);
                let mut bytes: Vec<u8> = Vec::with_capacity(len);
                if response
                    .into_reader()
                    .take(len as u64)
                    .read_to_end(&mut bytes)
                    .is_ok()
                {
                    match asset_type {
                        SyncAssetType::Mesh => {
                            let mut lock = meshes_to_apply.write();
                            loop {
                                match lock {
                                    Ok(mut map) => {
                                        debug!("Received mesh {} with size {}", id, len);
                                        map.insert(id, bytes);
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
                    map.insert(*id, mesh_to_bin(mesh));
                    break;
                }
                Err(_) => lock = self.meshes.write(),
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        format!("{}/mesh/{}", self.base_url, &id.to_string())
    }

    fn respond(rx: Receiver<Request>, meshes: MeshCache, max_size: usize) {
        for request in rx.iter() {
            let url = request.url();
            let Some(id) = url.strip_prefix("/mesh/") else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(id) else {
                continue;
            };

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
            debug!("Responding to {} with size {}", url, mesh.len());
            request
                .respond(Response::from_data(mesh.clone()).with_chunked_threshold(max_size))
                .unwrap_or(());
        }
    }
}
