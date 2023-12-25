use std::net::{ToSocketAddrs, SocketAddr};

use bevy::prelude::*;
use threadpool::ThreadPool;
use tiny_http::Server;

#[derive(Resource)]
struct SyncAssetTransfer {
    pool: ThreadPool,
    server: Server,
}

impl SyncAssetTransfer {
    fn new<A: ToSocketAddrs>(bind: A, thread_count: usize) -> Self {
        let pool = ThreadPool::new(thread_count);
        let server = Server::http(bind).unwrap();
        Self { pool, server }
    }
}

pub(crate) fn setup(app: &mut App, addr: std::net::IpAddr, port: u16) {
    let thread_count = 1;
    debug!("Initializing asset sync on {:?}:{}, parallel: {}", addr, port, thread_count);
    let http_transfer = SyncAssetTransfer::new(SocketAddr::new(addr, port), thread_count);
    app.insert_resource(http_transfer);
}

