mod image_serde;
mod mesh_serde;

use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        mpsc::{channel, Receiver},
        Arc, RwLock,
    },
    time::Duration,
};

use crate::{
    lib_priv::SyncTrackerRes, networking::assets::image_serde::image_to_bin, proto::SyncAssetType,
};
use ascii::AsciiString;
use bevy::{prelude::*, utils::HashMap};
use mesh_serde::{bin_to_mesh, mesh_to_bin};
use std::io::Read;
use threadpool::ThreadPool;
use tiny_http::{Header, Request, Response, Server};
use uuid::Uuid;

use self::image_serde::bin_to_image;

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
        process_mesh_assets.run_if(resource_exists::<SyncAssetTransfer>),
    );
    app.add_systems(
        Update,
        process_image_assets.run_if(resource_exists::<SyncAssetTransfer>),
    );
    app.add_systems(
        Update,
        process_audio_assets.run_if(resource_exists::<SyncAssetTransfer>),
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

fn process_image_assets(
    mut images: ResMut<Assets<Image>>,
    sync: ResMut<SyncAssetTransfer>,
    mut sync_tracker: ResMut<SyncTrackerRes>,
) {
    let Ok(mut map) = sync.images_to_apply.write() else {
        return;
    };
    for (id, image) in map.drain() {
        sync_tracker.pushed_handles_from_network.insert(id);
        let id: AssetId<Image> = AssetId::Uuid { uuid: id };
        let Some(img) = bin_to_image(&image) else {
            continue;
        };
        images.insert(id, img);
    }
}

fn process_audio_assets(
    mut audios: ResMut<Assets<AudioSource>>,
    sync: ResMut<SyncAssetTransfer>,
    mut sync_tracker: ResMut<SyncTrackerRes>,
) {
    let Ok(mut map) = sync.audios_to_apply.write() else {
        return;
    };
    for (id, audio) in map.drain() {
        sync_tracker.pushed_handles_from_network.insert(id);
        let id: AssetId<AudioSource> = AssetId::Uuid { uuid: id };
        audios.insert(
            id,
            AudioSource {
                bytes: audio.into(),
            },
        );
    }
}

type MeshCache = Arc<RwLock<HashMap<Uuid, Vec<u8>>>>;
type ImageCache = Arc<RwLock<HashMap<Uuid, Vec<u8>>>>;
type AudioCache = Arc<RwLock<HashMap<Uuid, Vec<u8>>>>;

#[derive(Resource)]
pub(crate) struct SyncAssetTransfer {
    base_url: String,
    server_pool: ThreadPool,
    download_pool: ThreadPool,
    meshes: MeshCache,
    meshes_to_apply: MeshCache,
    images: ImageCache,
    images_to_apply: ImageCache,
    audios: AudioCache,
    audios_to_apply: AudioCache,
    max_transfer: usize,
}

impl SyncAssetTransfer {
    pub(crate) fn new(addr: IpAddr, port: u16, max_transfer: usize) -> Self {
        let base_url = if addr.is_ipv6() {
            format!("http://[{}]:{}", addr, port)
        } else {
            format!("http://{}:{}", addr, port)
        };
        debug!("Starting asset server on {}", base_url);
        let server = Server::http(SocketAddr::new(addr, port)).unwrap();

        let server_pool = ThreadPool::new(128);
        let download_pool = ThreadPool::new(127);

        let meshes = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let meshes_to_apply = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let images = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let images_to_apply = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let audios = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));
        let audios_to_apply = Arc::new(RwLock::new(HashMap::<Uuid, Vec<u8>>::new()));

        let result = Self {
            base_url,
            server_pool,
            download_pool,
            meshes,
            meshes_to_apply,
            max_transfer,
            images,
            images_to_apply,
            audios,
            audios_to_apply,
        };

        let (server_tx, server_rx) = channel::<Request>();
        result.server_pool.execute(move || {
            for request in server.incoming_requests() {
                debug!("Queuing response to {}", request.url());
                server_tx.send(request).unwrap_or(());
            }
        });
        let meshes = result.meshes.clone();
        let images = result.images.clone();
        let audios = result.audios.clone();
        result
            .server_pool
            .execute(move || Self::respond(server_rx, meshes, images, audios, max_transfer));
        result
    }

    pub(crate) fn request(&self, asset_type: SyncAssetType, id: Uuid, url: String) {
        if let Ok(meshes) = self.meshes.read() {
            if meshes.contains_key(&id) {
                return;
            }
        }
        let meshes_to_apply = self.meshes_to_apply.clone();
        let images_to_apply = self.images_to_apply.clone();
        let audios_to_apply = self.audios_to_apply.clone();
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
                        SyncAssetType::Image => {
                            let mut lock = images_to_apply.write();
                            loop {
                                match lock {
                                    Ok(mut map) => {
                                        debug!("Received image {} with size {}", id, len);
                                        map.insert(id, bytes);
                                        break;
                                    }
                                    Err(_) => lock = images_to_apply.write(),
                                }
                                std::thread::sleep(Duration::from_millis(1));
                            }
                        }
                        SyncAssetType::Audio => {
                            let mut lock = audios_to_apply.write();
                            loop {
                                match lock {
                                    Ok(mut map) => {
                                        debug!("Received audio {} with size {}", id, len);
                                        map.insert(id, bytes);
                                        break;
                                    }
                                    Err(_) => lock = audios_to_apply.write(),
                                }
                                std::thread::sleep(Duration::from_millis(1));
                            }
                        }
                    }
                }
            }
        });
    }

    pub(crate) fn serve_mesh(&mut self, id: &Uuid, mesh: &Mesh) -> String {
        let mut lock = self.meshes.write();
        loop {
            match lock {
                Ok(mut map) => {
                    let mesh = map.entry(*id).or_insert_with(|| mesh_to_bin(mesh));
                    debug!("Serving mesh {} with size {}", id, mesh.len());
                    break;
                }
                Err(_) => lock = self.meshes.write(),
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        format!("{}/mesh/{}", self.base_url, &id.to_string())
    }

    pub(crate) fn serve_image(&mut self, id: &Uuid, image: &Image) -> String {
        let mut lock = self.images.write();
        loop {
            match lock {
                Ok(mut map) => {
                    if let Some(bin) = image_to_bin(image) {
                        let image = map.entry(*id).or_insert_with(|| bin);
                        debug!("Serving image {} with size {}", id, image.len());
                    }
                    break;
                }
                Err(_) => lock = self.meshes.write(),
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        format!("{}/image/{}", self.base_url, &id.to_string())
    }

    pub(crate) fn serve_audio(&mut self, id: &Uuid, audio: &AudioSource) -> String {
        let mut lock = self.audios.write();
        loop {
            match lock {
                Ok(mut map) => {
                    let bin = Vec::<u8>::from(audio.as_ref());
                    let audio = map.entry(*id).or_insert_with(|| bin);
                    debug!("Serving audio {} with size {}", id, audio.len());
                    break;
                }
                Err(_) => lock = self.meshes.write(),
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        format!("{}/audio/{}", self.base_url, &id.to_string())
    }

    fn respond(
        rx: Receiver<Request>,
        meshes: MeshCache,
        images: ImageCache,
        audios: AudioCache,
        max_size: usize,
    ) {
        for request in rx.iter() {
            let url = request.url();
            let (asset_type, id) = if url.contains("/image/") {
                let Some(id) = url.strip_prefix("/image/") else {
                    continue;
                };
                (SyncAssetType::Image, id)
            } else if url.contains("/mesh/") {
                let Some(id) = url.strip_prefix("/mesh/") else {
                    continue;
                };
                (SyncAssetType::Mesh, id)
            } else if url.contains("/audio/") {
                let Some(id) = url.strip_prefix("/audio/") else {
                    continue;
                };
                (SyncAssetType::Audio, id)
            } else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(id) else {
                continue;
            };

            match asset_type {
                SyncAssetType::Mesh => {
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
                        .respond(
                            Response::from_data(mesh.clone())
                                .with_header(Header {
                                    field: "Content-Length".parse().unwrap(),
                                    value: AsciiString::from_ascii(mesh.len().to_string()).unwrap(),
                                })
                                .with_chunked_threshold(max_size),
                        )
                        .unwrap_or(());
                }
                SyncAssetType::Image => {
                    let Ok(imagesmap) = images.read() else {
                        request
                            .respond(Response::from_string("").with_status_code(449))
                            .unwrap_or(());
                        continue;
                    };
                    let Some(image) = imagesmap.get(&id) else {
                        request
                            .respond(Response::from_string("").with_status_code(404))
                            .unwrap_or(());
                        continue;
                    };
                    debug!("Responding to {} with size {}", url, image.len());
                    request
                        .respond(
                            Response::from_data(image.clone())
                                .with_header(Header {
                                    field: "Content-Length".parse().unwrap(),
                                    value: AsciiString::from_ascii(image.len().to_string())
                                        .unwrap(),
                                })
                                .with_chunked_threshold(max_size),
                        )
                        .unwrap_or(());
                }
                SyncAssetType::Audio => {
                    let Ok(audiosmap) = audios.read() else {
                        request
                            .respond(Response::from_string("").with_status_code(449))
                            .unwrap_or(());
                        continue;
                    };
                    let Some(audio) = audiosmap.get(&id) else {
                        request
                            .respond(Response::from_string("").with_status_code(404))
                            .unwrap_or(());
                        continue;
                    };
                    debug!("Responding to {} with size {}", url, audio.len());
                    request
                        .respond(
                            Response::from_data(audio.clone())
                                .with_header(Header {
                                    field: "Content-Length".parse().unwrap(),
                                    value: AsciiString::from_ascii(audio.len().to_string())
                                        .unwrap(),
                                })
                                .with_chunked_threshold(max_size),
                        )
                        .unwrap_or(());
                }
            }
        }
    }
}
