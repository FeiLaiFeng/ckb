use crate::helper::{deadlock_detection, wait_for_exit};
use crate::setup::{ExitCode, RunArgs};
use ckb_chain::chain::{ChainBuilder, ChainController};
use ckb_db::diskdb::RocksDB;
use ckb_miner::BlockAssembler;
use ckb_network::{CKBProtocol, NetworkService, NetworkState, ProtocolId};
use ckb_notify::{NotifyController, NotifyService};
use ckb_rpc::RpcServer;
use ckb_shared::cachedb::CacheDB;
use ckb_shared::index::ChainIndex;
use ckb_shared::shared::{Shared, SharedBuilder};
use ckb_sync::{NetTimeProtocol, NetworkProtocol, Relayer, Synchronizer};
use ckb_traits::chain_provider::ChainProvider;
use log::info;
use std::sync::Arc;

pub fn run(args: RunArgs) -> Result<(), ExitCode> {
    deadlock_detection();

    let shared = SharedBuilder::<CacheDB<RocksDB>>::default()
        .consensus(args.consensus)
        .db(&args.config.db)
        .tx_pool_config(args.config.tx_pool)
        .build();

    let notify = NotifyService::default().start(Some("notify"));

    let chain_controller = setup_chain(shared.clone(), notify.clone());
    info!(target: "main", "chain genesis hash: {:#x}", shared.genesis_hash());

    let block_assembler = BlockAssembler::new(shared.clone(), args.config.block_assembler);
    let block_assembler_controller = block_assembler.start(Some("MinerAgent"), &notify);

    let synchronizer =
        Synchronizer::new(chain_controller.clone(), shared.clone(), args.config.sync);

    let relayer = Relayer::new(
        chain_controller.clone(),
        shared.clone(),
        synchronizer.peers(),
    );

    let net_time_checker = NetTimeProtocol::default();

    let network_state = Arc::new(
        NetworkState::from_config(args.config.network).expect("Init network state failed"),
    );
    let protocols = vec![
        CKBProtocol::new(
            "syn".to_string(),
            NetworkProtocol::SYNC as ProtocolId,
            &[1][..],
            Box::new(synchronizer),
            Arc::clone(&network_state),
        ),
        CKBProtocol::new(
            "rel".to_string(),
            NetworkProtocol::RELAY as ProtocolId,
            &[1][..],
            Box::new(relayer),
            Arc::clone(&network_state),
        ),
        CKBProtocol::new(
            "tim".to_string(),
            NetworkProtocol::TIME as ProtocolId,
            &[1][..],
            Box::new(net_time_checker),
            Arc::clone(&network_state),
        ),
    ];
    let network_controller = NetworkService::new(Arc::clone(&network_state), protocols)
        .start(Some("NetworkService"))
        .expect("Start network service failed");

    let rpc_server = RpcServer::new(
        args.config.rpc,
        network_controller,
        shared,
        chain_controller,
        block_assembler_controller,
    );

    wait_for_exit();

    info!(target: "main", "Finishing work, please wait...");

    rpc_server.close();
    info!(target: "main", "Jsonrpc shutdown");

    Ok(())
}

fn setup_chain<CI: ChainIndex + 'static>(
    shared: Shared<CI>,
    notify: NotifyController,
) -> ChainController {
    let chain_service = ChainBuilder::new(shared, notify).build();
    chain_service.start(Some("ChainService"))
}