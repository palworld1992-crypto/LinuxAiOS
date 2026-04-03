//! Raft consensus engine with gRPC network transport.

use crate::proto::raft_service_client::RaftServiceClient;
use crate::proto::{
    AppendEntriesRequest as GrpcAppendEntries, InstallSnapshotRequest as GrpcInstallSnapshot,
    RaftVoteRequest as GrpcVoteRequest,
};
use crate::storage::{LogData, NodeId, RaftStorageImpl, SnapshotData};
use anyhow::Result;
use async_trait::async_trait;
use openraft::{
    error::{NetworkError, RPCError, RaftError},
    network::RPCOption,
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
        InstallSnapshotResponse, VoteRequest, VoteResponse,
    },
    storage::Adaptor,
    Config, Raft, RaftNetwork, RaftNetworkFactory, RaftTypeConfig, Vote,
};
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::Channel;

// -----------------------------------------------------------------------------
// RaftTypeConfig
// -----------------------------------------------------------------------------
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct RaftTypeConfigImpl;

impl RaftTypeConfig for RaftTypeConfigImpl {
    type D = LogData;
    type R = LogData;
    type NodeId = NodeId;
    type Entry = openraft::Entry<Self>;
    type SnapshotData = SnapshotData;
    type Node = ();
}

// -----------------------------------------------------------------------------
// ConsensusEngine
// -----------------------------------------------------------------------------
pub struct ConsensusEngine {
    pub raft: Raft<
        RaftTypeConfigImpl,
        NetworkFactory,
        Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
        Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
    >,
}

impl ConsensusEngine {
    pub async fn new(node_id: NodeId, peers_vec: Vec<String>) -> Result<Self> {
        let config = Config {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            ..Default::default()
        };

        let db_path = format!("master_{}.db", node_id);
        let storage = RaftStorageImpl::new(&db_path, node_id)?;
        let network = NetworkFactory::new(peers_vec).await?;
        let (log_store, sm_store) = Adaptor::new(storage);

        let raft = Raft::new(node_id, Arc::new(config), network, log_store, sm_store).await?;

        Ok(Self { raft })
    }

    pub async fn submit_proposal(&self, data: LogData) -> Result<()> {
        self.raft.client_write(data).await?;
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// NetworkFactory
// -----------------------------------------------------------------------------
pub struct NetworkFactory {
    peers: HashMap<NodeId, String>,
}

impl NetworkFactory {
    pub async fn new(peer_addrs: Vec<String>) -> Result<Self> {
        let mut peers = HashMap::new();
        for addr in peer_addrs {
            let parts: Vec<&str> = addr.splitn(2, ':').collect();
            if parts.len() == 2 {
                if let Ok(id) = parts[0].parse::<NodeId>() {
                    let endpoint = if parts[1].starts_with("http") {
                        parts[1].to_string()
                    } else {
                        format!("http://{}", parts[1])
                    };
                    peers.insert(id, endpoint);
                }
            }
        }
        Ok(Self { peers })
    }
}

#[async_trait]
impl RaftNetworkFactory<RaftTypeConfigImpl> for NetworkFactory {
    type Network = NetworkClient;

    async fn new_client(&mut self, target: NodeId, _node: &()) -> Self::Network {
        let address = self.peers.get(&target).cloned().unwrap_or_default();
        NetworkClient { target, address }
    }
}

// -----------------------------------------------------------------------------
// NetworkClient
// -----------------------------------------------------------------------------
#[derive(Clone)]
pub struct NetworkClient {
    target: NodeId,
    address: String,
}

impl NetworkClient {
    async fn get_client(&self) -> Result<RaftServiceClient<Channel>, NetworkError> {
        if self.address.is_empty() {
            return Err(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                "Empty address",
            )));
        }

        let channel = Channel::from_shared(self.address.clone())
            .map_err(|e| NetworkError::new(&std::io::Error::new(std::io::ErrorKind::Other, e)))?
            .connect()
            .await
            .map_err(|e| NetworkError::new(&std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(RaftServiceClient::new(channel))
    }
}

#[async_trait]
impl RaftNetwork<RaftTypeConfigImpl> for NetworkClient {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<RaftTypeConfigImpl>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, (), RaftError<NodeId>>> {
        let mut client = self.get_client().await.map_err(RPCError::Network)?;

        let request = GrpcAppendEntries {
            term: rpc.vote.leader_id.term,
            leader_id: rpc.vote.leader_id.node_id,
            prev_log_index: rpc.prev_log_id.as_ref().map(|id| id.index).unwrap_or(0),
            prev_log_term: rpc
                .prev_log_id
                .as_ref()
                .map(|id| id.leader_id.term)
                .unwrap_or(0),
            entries: rpc
                .entries
                .into_iter()
                .map(|entry| crate::proto::Entry {
                    index: entry.log_id.index,
                    term: entry.log_id.leader_id.term,
                    data: match entry.payload {
                        openraft::EntryPayload::Normal(log_data) => log_data,
                        _ => vec![],
                    },
                })
                .collect(),
            leader_commit: rpc.leader_commit.map(|id| id.index).unwrap_or(0),
        };

        let response = client.append_entries(request).await.map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })?;

        let resp = response.into_inner();

        // SỬA LỖI E0559: Trong openraft 0.8.8, Success và HigherVote là Tuple Variants
        if resp.success {
            Ok(AppendEntriesResponse::Success)
        } else {
            Ok(AppendEntriesResponse::HigherVote(Vote::new(
                resp.term,
                self.target,
            )))
        }
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<RaftTypeConfigImpl>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, (), RaftError<NodeId, openraft::error::InstallSnapshotError>>,
    > {
        let mut client = self.get_client().await.map_err(RPCError::Network)?;

        let request = GrpcInstallSnapshot {
            term: rpc.vote.leader_id.term,
            leader_id: rpc.vote.leader_id.node_id,
            last_included_index: rpc
                .meta
                .last_log_id
                .as_ref()
                .map(|id| id.index)
                .unwrap_or(0),
            last_included_term: rpc
                .meta
                .last_log_id
                .as_ref()
                .map(|id| id.leader_id.term)
                .unwrap_or(0),
            offset: rpc.offset,
            data: rpc.data,
            done: rpc.done,
        };

        let response = client.install_snapshot(request).await.map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })?;

        let resp = response.into_inner();

        Ok(InstallSnapshotResponse {
            vote: Vote::new(resp.term, self.target),
        })
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, (), RaftError<NodeId>>> {
        let mut client = self.get_client().await.map_err(RPCError::Network)?;

        let request = GrpcVoteRequest {
            term: rpc.vote.leader_id.term,
            candidate_id: rpc.vote.leader_id.node_id,
            last_log_index: rpc.last_log_id.as_ref().map(|id| id.index).unwrap_or(0),
            last_log_term: rpc
                .last_log_id
                .as_ref()
                .map(|id| id.leader_id.term)
                .unwrap_or(0),
        };

        let response = client.vote(request).await.map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                e,
            )))
        })?;

        let resp = response.into_inner();
        Ok(VoteResponse {
            vote: Vote::new(resp.term, self.target),
            vote_granted: resp.vote_granted,
            last_log_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_type_config_impl() {
        let _config = RaftTypeConfigImpl;
        let node_id: <RaftTypeConfigImpl as RaftTypeConfig>::NodeId = 1;
        assert_eq!(node_id, 1);
    }

    #[tokio::test]
    async fn test_network_factory_new() {
        let peer_addrs = vec![
            "1:localhost:8001".to_string(),
            "2:localhost:8002".to_string(),
        ];

        let factory = NetworkFactory::new(peer_addrs).await;
        assert!(factory.is_ok());

        let factory = factory.unwrap();
        assert!(factory.peers.contains_key(&1));
        assert!(factory.peers.contains_key(&2));
    }

    #[test]
    fn test_network_factory_parse() {
        let peer_addrs = vec![
            "0:http://localhost:8001".to_string(),
            "1:http://localhost:8002".to_string(),
        ];

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async {
            let factory = NetworkFactory::new(peer_addrs).await.unwrap();
            assert_eq!(factory.peers.get(&0).unwrap(), "http://localhost:8001");
            assert_eq!(factory.peers.get(&1).unwrap(), "http://localhost:8002");
        });
    }
}
