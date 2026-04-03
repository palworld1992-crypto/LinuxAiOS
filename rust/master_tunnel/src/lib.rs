pub mod blockchain;
pub mod consensus;
pub mod ledger;
pub mod raft_service;
pub mod storage;
pub mod supervisor;
pub mod types;

use crate::consensus::ConsensusEngine;
use crate::ledger::Ledger;
use crate::supervisor::{SupervisorInfo, SupervisorRegistry, SupervisorType};
// Xóa: use crate::blockchain::Block; (Tránh warning unused import)
use crate::storage::LogData;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/master.rs"));
}
use proto::master_service_server::{MasterService, MasterServiceServer};
use proto::raft_service_server::RaftServiceServer;
use proto::{
    GetLedgerRequest, GetLedgerResponse, ProposalRequest, ProposalResponse, RegisterRequest,
    RegisterResponse, VoteRequest as ProtoVoteRequest, VoteResponse as ProtoVoteResponse,
};

#[derive(Clone)]
pub struct MasterTunnel {
    consensus: Arc<ConsensusEngine>,
    registry: Arc<SupervisorRegistry>,
    ledger: Arc<Ledger>,
    master_kyber_public: Vec<u8>,
    master_dilithium_public: Vec<u8>,
}

impl MasterTunnel {
    pub async fn new(node_id: u64, peers: Vec<String>) -> anyhow::Result<Self> {
        let consensus = Arc::new(ConsensusEngine::new(node_id, peers).await?);
        let registry = Arc::new(SupervisorRegistry::new());
        let ledger = Arc::new(Ledger::new());

        // Khởi tạo key mặc định hoặc load từ config
        let master_kyber_public = vec![0u8; 1568];
        let master_dilithium_public = vec![0u8; 1952];

        Ok(Self {
            consensus,
            registry,
            ledger,
            master_kyber_public,
            master_dilithium_public,
        })
    }

    pub async fn run(self, addr: std::net::SocketAddr) -> anyhow::Result<()> {
        let master_service = MasterServiceServer::new(self.clone());

        // Lấy raft instance từ consensus engine
        let raft_instance = self.consensus.raft.clone();
        let raft_service =
            RaftServiceServer::new(crate::raft_service::RaftServiceImpl::new(raft_instance));

        println!("🚀 Master Server (Node) listening on {}", addr);
        Server::builder()
            .add_service(master_service)
            .add_service(raft_service)
            .serve(addr)
            .await?;
        Ok(())
    }
}

#[tonic::async_trait]
impl MasterService for MasterTunnel {
    async fn submit_proposal(
        &self,
        request: Request<ProposalRequest>,
    ) -> Result<Response<ProposalResponse>, Status> {
        let req = request.into_inner();

        // SỬA LỖI E0560: LogData là Vec<u8>, không dùng LogData { data: ... }
        let log_data: LogData = req.data;

        self.consensus
            .submit_proposal(log_data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ProposalResponse {
            accepted: true,
            message: "Proposal submitted to Raft".into(),
        }))
    }

    async fn register_supervisor(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let _req = request.into_inner();

        let (kyber_pub, kyber_priv) = scc::crypto::kyber_keypair()
            .map_err(|e| Status::internal(format!("Kyber Error: {}", e)))?;
        let (dilithium_pub, dilithium_priv) = scc::crypto::dilithium_keypair()
            .map_err(|e| Status::internal(format!("Dilithium Error: {}", e)))?;

        let id = self
            .registry
            .register_core(
                SupervisorType::Linux,
                SupervisorInfo {
                    id: 0,
                    supervisor_type: SupervisorType::Linux,
                    public_key_kyber: kyber_pub.to_vec(),
                    public_key_dilithium: dilithium_pub.to_vec(),
                    is_standby: false,
                    registered_at: common::utils::current_timestamp_ms(),
                },
            )
            .map_err(Status::already_exists)?;

        Ok(Response::new(RegisterResponse {
            supervisor_id: id,
            kyber_private_key: kyber_priv.to_vec(),
            dilithium_private_key: dilithium_priv.to_vec(),
            master_kyber_public: self.master_kyber_public.clone(),
            master_dilithium_public: self.master_dilithium_public.clone(),
        }))
    }

    async fn get_ledger(
        &self,
        _: Request<GetLedgerRequest>,
    ) -> Result<Response<GetLedgerResponse>, Status> {
        let ledger_data =
            bincode::serialize(&*self.ledger).map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetLedgerResponse {
            block_data: ledger_data,
        }))
    }

    async fn submit_vote(
        &self,
        request: Request<ProtoVoteRequest>,
    ) -> Result<Response<ProtoVoteResponse>, Status> {
        let vote = request.into_inner();

        let vote_bytes = bincode::serialize(&vote).map_err(|e| Status::internal(e.to_string()))?;

        // SỬA LỖI E0560: LogData là Vec<u8>
        let log_data: LogData = vote_bytes;

        self.consensus
            .submit_proposal(log_data)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ProtoVoteResponse { success: true }))
    }
}
