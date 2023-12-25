use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
};

use bevy::{prelude::*, utils::HashMap};
use std::io::Read;
use threadpool::ThreadPool;
use tiny_http::{Request, Response, Server};

use crate::mesh_serde::{mesh_to_bin, bin_to_mesh};

const MAX_BYTES: u64 = 100_000_000;

#[derive(Resource)]
pub(crate) struct SyncAssetTransfer {
    server_pool: ThreadPool,
    download_pool: ThreadPool,
    mesh_tx: Sender<Mesh>,
    meshes: Arc<HashMap<String, Mesh>>,
}

pub(crate) enum SyncAssetType {
    Mesh,
}

impl SyncAssetTransfer {
    fn new<A: ToSocketAddrs>(bind: A) -> Self {
        let server_pool = ThreadPool::new(2);
        let download_pool = ThreadPool::new(127);
        let meshes = Arc::new(HashMap::<String, Mesh>::new());

        let (server_tx, server_rx) = channel::<Request>();
        let (mesh_tx, mesh_rx) = channel::<Mesh>();
        let server = Server::http(bind).unwrap();
        let result = Self {
            server_pool,
            download_pool,
            mesh_tx,
            meshes,
        };
        result.server_pool.execute(move || {
            for request in server.incoming_requests() {
                server_tx.send(request).unwrap_or(());
            }
        });
        let meshes = result.meshes.clone();
        result
            .server_pool
            .execute(|| Self::respond(server_rx, meshes));

        result
    }

    pub(crate) fn queue(&self, asset_type: SyncAssetType, id: String, url: String) {
        if self.meshes.contains_key(&id) {
            return;
        }
        let tx = self.mesh_tx.clone();
        self.download_pool.execute(move || {
            if let Ok(response) = ureq::get(url.as_str()).call() {
                let len = response
                    .header("Content-Length")
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap();
                let mut bytes: Vec<u8> = Vec::with_capacity(len);
                if response
                    .into_reader()
                    .take(MAX_BYTES)
                    .read_to_end(&mut bytes)
                    .is_ok()
                {
                    match asset_type {
                        SyncAssetType::Mesh => {
                            let mesh = bin_to_mesh(bytes.as_slice());
                            tx.send(mesh).unwrap_or(());
                        }
                    }
                }
            }
        });
    }

    fn respond(rx: Receiver<Request>, meshes: Arc<HashMap<String, Mesh>>) {
        for request in rx.iter() {
            let url = request.url();
            let Some(id) = url.strip_prefix("/mesh/") else {
                continue;
            };

            let Some(mesh) = meshes.get(id) else {
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

pub(crate) fn setup(app: &mut App, addr: std::net::IpAddr, port: u16) {
    let thread_count = 1;
    debug!(
        "Initializing asset sync on {:?}:{}, parallel: {}",
        addr, port, thread_count
    );
    let http_transfer = SyncAssetTransfer::new(SocketAddr::new(addr, port));
    app.insert_resource(http_transfer);
}
