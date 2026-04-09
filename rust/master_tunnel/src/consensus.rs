//! Raft consensus engine with gRPC network transport.

use crate::proto::raft_service_client::RaftServiceClient;
use crate::proto::{
    AppendEntriesRequest as GrpcAppendEntries,
    RaftVoteRequest as GrpcVoteRequest,
};
use crate::storage::{LogData, NodeId, RaftStorageImpl, SnapshotData};
use anyhow::Result;
use async_trait::async_trait;
use openraft::{
    error::{NetworkError, RPCError, RaftError},
    network::RPCOption,
    raft::{
        AppendEntriesRequest, AppendEntriesResponse, VoteRequest, VoteResponse,
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
        let address = match self.peers.get(&target) {
            Some(addr) => addr.clone(),
            None => {
                tracing::warn!("No peer address found for target node: {}", target);
                String::new()
            }
        };
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
            .map_err(|e| NetworkError::new(&std::io::Error::other(e)))?
            .connect()
            .await
            .map_err(|e| NetworkError::new(&std::io::Error::other(e)))?;
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

        let prev_log_index = match rpc.prev_log_id.as_ref() {
            Some(id) => id.index,
            None => 0, // default index khi không có prev_log_id
        };
        let prev_log_term = match rpc.prev_log_id.as_ref() {
            Some(id) => id.leader_id.term,
            None => Default::default(), // default term
        };
        let leader_commit = match rpc.leader_commit.as_ref() {
            Some(id) => id.index,
            None => 0, // default commit index
        };

        let entries: Vec<crate::proto::Entry> = rpc
            .entries
            .iter()
            .map(|e| crate::proto::Entry {
                index: e.log_id.index,
                term: e.log_id.leader_id.term,
                data: match &e.payload {
                    openraft::EntryPayload::Normal(d) => d.clone(),
                    openraft::EntryPayload::Blank => vec![],
                    openraft::EntryPayload::Membership(_) => vec![],
                },
            })
            .collect();

        let request = GrpcAppendEntries {
            term: rpc.vote.leader_id.term,
            leader_id: rpc.vote.leader_id.node_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        };

        let response = client.append_entries(request).await.map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::other(e)))
        })?;

        let _resp = response.into_inner();

        Ok(AppendEntriesResponse::Success)
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
            last_log_index: match rpc.last_log_id.as_ref() {
                Some(id) => id.index,
                None => 0,
            },
            last_log_term: match rpc.last_log_id.as_ref() {
                Some(id) => id.leader_id.term,
                None => Default::default(),
            },
        };

        let response = client.vote(request).await.map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::other(
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
    async fn test_network_factory_new() -> anyhow::Result<()> {
        let peer_addrs = vec![
            "1:localhost:8001".to_string(),
            "2:localhost:8002".to_string(),
        ];

        let factory = NetworkFactory::new(peer_addrs).await?;
        assert!(factory.peers.contains_key(&1));
        assert!(factory.peers.contains_key(&2));
        Ok(())
    }

    #[tokio::test]
    async fn test_network_factory_parse() -> anyhow::Result<()> {
        let peer_addrs = vec![
            "0:http://localhost:8001".to_string(),
            "1:http://localhost:8002".to_string(),
        ];

        let factory = NetworkFactory::new(peer_addrs).await?;
        assert_eq!(factory.peers.get(&0), Some(&"http://localhost:8001".to_string()));
        assert_eq!(factory.peers.get(&1), Some(&"http://localhost:8002".to_string()));
        Ok(())
    }
}
