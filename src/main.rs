mod rpc;

use argh::FromArgs;
use axum::routing::{get, post};
use axum::Router;
use hex::prelude::*;
use ldk_node::bitcoin::Network;
use ldk_node::lightning::util::logger::{Logger, Record};
use ldk_node::{Builder, Node};
use rand::{thread_rng, RngCore};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct TracingLogger {}

impl Logger for TracingLogger {
    fn log(&self, record: Record) {
        tracing::info!("{:?}", record);
    }
}

#[derive(FromArgs)]
/// Lightning Relay Sample Node
struct Args {
    #[argh(option)]
    /// the data dir for storing ldk data
    data_dir: String,
    #[argh(option)]
    /// the port the node listens for control plane rpc on.
    rpc_port: u16,
    #[argh(option)]
    /// the port the node listens for requests from relays on.
    node_service_port: u16,
    #[argh(option)]
    /// esplora url.
    esplora_url: String,
    #[argh(option)]
    /// rgs url.
    rgs_url: String,
    #[argh(option)]
    /// network.
    network: Network,
    #[argh(option)]
    /// node seed bytes as hex string.
    /// if not provided, one will be generated and written to stdout.
    seed_hex: Option<String>,
}

#[derive(Clone)]
struct AppState {
    node: Arc<Node>,
}

fn main() {
		tracing_subscriber::fmt::init();

    let args: Args = argh::from_env();
    let node_service_addr: SocketAddr = format!("[::]:{}", args.node_service_port).parse().unwrap();

    let seed_bytes = args
        .seed_hex
        .map(|seed_hex| <[u8; 64]>::from_hex(&seed_hex).expect("valid 64 byte hex seed"))
        .unwrap_or_else(|| {
            let mut seed = [0u8; 64];
            thread_rng().fill_bytes(&mut seed);
            println!(
                "no seed provided, generated new seed: {}",
                seed.to_lower_hex_string()
            );
            seed
        });

    let node = Builder::new()
    	.set_network(args.network)
    	.set_relay_node_address(node_service_addr.into()).unwrap()
    	.set_esplora_server(args.esplora_url)
   		.set_gossip_source_rgs(args.rgs_url)
    	.set_entropy_seed_bytes(seed_bytes.to_vec()).unwrap()
			.set_storage_dir_path(args.data_dir)
    	.build_with_fs_store().unwrap();

		println!("node id: {}", node.node_id().to_string());

    node.start().unwrap();

    let app_state = AppState {
        node: Arc::new(node),
    };

    let app = Router::new()
				.route("/connect-peer", post(rpc::connect_peer))
				.route("/peers", get(rpc::list_peers))
        .route("/funding-address", get(rpc::funding_address))
        .route("/channels", post(rpc::open_channel))
        .route("/channels", get(rpc::list_channels))
        .route("/pay-invoice", post(rpc::pay_invoice))
        .route("/get-invoice", post(rpc::get_invoice))
        .route("/sync", post(rpc::sync))
        .route("/balance", get(rpc::get_balance))
        .route("/get-payment/:payment_hash", get(rpc::get_payment))
        .with_state(app_state);

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.rpc_port))
            .await
            .unwrap();

        println!("started http server listening on port: {}", args.rpc_port);

        axum::serve(listener, app).await.unwrap();
    });
}
