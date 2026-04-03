use tonic::{Request, Response, Status};

// Import các trait và struct từ proto được generate
use crate::proto::raft_service_server::RaftService;
use crate::proto::{
    AppendEntriesRequest as ProtoAppendEntriesRequest,
    AppendEntriesResponse as ProtoAppendEntriesResponse,
    InstallSnapshotRequest as ProtoInstallSnapshotRequest,
    InstallSnapshotResponse as ProtoInstallSnapshotResponse, RaftVoteRequest, RaftVoteResponse,
};

// Import từ local modules và openraft
use crate::consensus::NetworkFactory;
use crate::consensus::RaftTypeConfigImpl;
use crate::storage::RaftStorageImpl;
use openraft::storage::Adaptor;
use openraft::Raft;

/// Định nghĩa cấu trúc RaftServiceImpl để handle các gRPC call
pub struct RaftServiceImpl {
    pub raft: Raft<
        RaftTypeConfigImpl,
        NetworkFactory,
        Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
        Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
    >,
}

impl RaftServiceImpl {
    pub fn new(
        raft: Raft<
            RaftTypeConfigImpl,
            NetworkFactory,
            Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
            Adaptor<RaftTypeConfigImpl, RaftStorageImpl>,
        >,
    ) -> Self {
        Self { raft }
    }
}

#[tonic::async_trait]
impl RaftService for RaftServiceImpl {
    async fn append_entries(
        &self,
        request: Request<ProtoAppendEntriesRequest>,
    ) -> Result<Response<ProtoAppendEntriesResponse>, Status> {
        let req = request.into_inner();

        // Chuyển đổi entries từ proto sang định dạng OpenRaft
        let entries = req
            .entries
            .into_iter()
            .map(|e| openraft::Entry {
                log_id: openraft::LogId::new(
                    openraft::CommittedLeaderId::new(e.term, req.leader_id),
                    e.index,
                ),
                payload: openraft::EntryPayload::Normal(e.data),
            })
            .collect();

        let raft_req = openraft::raft::AppendEntriesRequest {
            vote: openraft::Vote::new(req.term, req.leader_id),
            prev_log_id: if req.prev_log_index > 0 {
                Some(openraft::LogId::new(
                    openraft::CommittedLeaderId::new(req.prev_log_term, req.leader_id),
                    req.prev_log_index,
                ))
            } else {
                None
            },
            entries,
            leader_commit: if req.leader_commit > 0 {
                Some(openraft::LogId::new(
                    openraft::CommittedLeaderId::new(req.term, req.leader_id),
                    req.leader_commit,
                ))
            } else {
                None
            },
        };

        let resp = self
            .raft
            .append_entries(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        // Xử lý triệt để Enum AppendEntriesResponse của OpenRaft v0.8.8
        match resp {
            // Success: Unit Variant (Không chứa dữ liệu)
            openraft::raft::AppendEntriesResponse::Success => {
                Ok(Response::new(ProtoAppendEntriesResponse {
                    term: req.term,
                    success: true,
                    last_log_index: 0,
                }))
            }
            // PartialSuccess: Tuple Variant chứa Option<LogId>
            openraft::raft::AppendEntriesResponse::PartialSuccess(log_id) => {
                Ok(Response::new(ProtoAppendEntriesResponse {
                    term: req.term,
                    success: true,
                    last_log_index: log_id.map(|l| l.index).unwrap_or(0),
                }))
            }
            // HigherVote: Tuple Variant chứa cấu trúc Vote
            openraft::raft::AppendEntriesResponse::HigherVote(vote) => {
                Ok(Response::new(ProtoAppendEntriesResponse {
                    term: vote.leader_id.term,
                    success: false,
                    last_log_index: 0,
                }))
            }
            // Conflict: Unit Variant (Xảy ra khi log không khớp)
            openraft::raft::AppendEntriesResponse::Conflict => {
                Ok(Response::new(ProtoAppendEntriesResponse {
                    term: req.term,
                    success: false,
                    last_log_index: 0,
                }))
            }
        }
    }

    async fn vote(
        &self,
        request: Request<RaftVoteRequest>,
    ) -> Result<Response<RaftVoteResponse>, Status> {
        let req = request.into_inner();

        let raft_req = openraft::raft::VoteRequest {
            vote: openraft::Vote::new(req.term, req.candidate_id),
            last_log_id: if req.last_log_index > 0 {
                Some(openraft::LogId::new(
                    openraft::CommittedLeaderId::new(req.last_log_term, req.candidate_id),
                    req.last_log_index,
                ))
            } else {
                None
            },
        };

        let resp = self
            .raft
            .vote(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(RaftVoteResponse {
            term: resp.vote.leader_id.term,
            vote_granted: resp.vote_granted,
        }))
    }

    async fn install_snapshot(
        &self,
        request: Request<ProtoInstallSnapshotRequest>,
    ) -> Result<Response<ProtoInstallSnapshotResponse>, Status> {
        let req = request.into_inner();

        let meta = openraft::SnapshotMeta {
            last_log_id: Some(openraft::LogId::new(
                openraft::CommittedLeaderId::new(req.last_included_term, req.leader_id),
                req.last_included_index,
            )),
            last_membership: openraft::StoredMembership::default(),
            snapshot_id: format!("{}-{}", req.last_included_term, req.last_included_index),
        };

        let raft_req = openraft::raft::InstallSnapshotRequest {
            vote: openraft::Vote::new(req.term, req.leader_id),
            meta,
            offset: req.offset,
            data: req.data,
            done: req.done,
        };

        let resp = self
            .raft
            .install_snapshot(raft_req)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(ProtoInstallSnapshotResponse {
            term: resp.vote.leader_id.term,
        }))
    }
}
