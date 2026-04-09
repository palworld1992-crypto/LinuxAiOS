use master_tunnel::types::{Message, Proposal, Vote};

#[test]
fn test_vote_creation() {
    let vote = Vote {
        proposal_id: 1,
        node_id: 5,
        approved: true,
        timestamp: 12345,
    };
    assert_eq!(vote.proposal_id, 1);
    assert_eq!(vote.node_id, 5);
    assert!(vote.approved);
}

#[test]
fn test_vote_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let vote = Vote {
        proposal_id: 42,
        node_id: 7,
        approved: false,
        timestamp: 99999,
    };
    let json = serde_json::to_string(&vote)?;
    let deserialized: Vote = serde_json::from_str(&json)?;
    assert_eq!(deserialized.proposal_id, 42);
    assert!(!deserialized.approved);
    Ok(())
}

#[test]
fn test_proposal_creation() {
    let proposal = Proposal {
        id: 100,
        data: vec![1, 2, 3, 4],
        proposer_id: 3,
        timestamp: 54321,
    };
    assert_eq!(proposal.id, 100);
    assert_eq!(proposal.data, vec![1, 2, 3, 4]);
    assert_eq!(proposal.proposer_id, 3);
}

#[test]
fn test_proposal_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let proposal = Proposal {
        id: 200,
        data: vec![0xAA, 0xBB],
        proposer_id: 1,
        timestamp: 11111,
    };
    let json = serde_json::to_string(&proposal)?;
    let deserialized: Proposal = serde_json::from_str(&json)?;
    assert_eq!(deserialized.id, 200);
    assert_eq!(deserialized.data, vec![0xAA, 0xBB]);
    Ok(())
}

#[test]
fn test_message_proposal_variant() {
    let proposal = Proposal {
        id: 1,
        data: vec![1],
        proposer_id: 1,
        timestamp: 1,
    };
    let msg = Message::Proposal(proposal);
    assert!(matches!(msg, Message::Proposal(_)));
}

#[test]
fn test_message_vote_variant() {
    let vote = Vote {
        proposal_id: 1,
        node_id: 1,
        approved: true,
        timestamp: 1,
    };
    let msg = Message::Vote(vote);
    assert!(matches!(msg, Message::Vote(_)));
}

#[test]
fn test_message_register_variant() {
    let msg = Message::Register {
        node_id: 5,
        address: "127.0.0.1:5000".to_string(),
    };
    assert!(matches!(msg, Message::Register { .. }));
}

#[test]
fn test_message_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let proposal = Proposal {
        id: 1,
        data: vec![1],
        proposer_id: 1,
        timestamp: 1,
    };
    let msg = Message::Proposal(proposal);
    let bytes = bincode::serialize(&msg)?;
    let deserialized: Message = bincode::deserialize(&bytes)?;
    assert!(matches!(deserialized, Message::Proposal(_)));
    Ok(())
}
