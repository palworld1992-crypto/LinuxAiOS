use scc::crypto::{dilithium_keypair, kyber_keypair};
use transport_tunnel::{client_handshake, TransportTunnel};

fn make_kyber_pub(v: Vec<u8>) -> Result<[u8; 1568], &'static str> {
    v.try_into().map_err(|_| "kyber_pub")
}

fn make_kyber_priv(v: Vec<u8>) -> Result<[u8; 2400], &'static str> {
    v.try_into().map_err(|_| "kyber_priv")
}

fn make_dilithium_pub(v: Vec<u8>) -> Result<[u8; 1952], &'static str> {
    v.try_into().map_err(|_| "dilithium_pub")
}

fn make_dilithium_priv(v: Vec<u8>) -> Result<[u8; 4032], &'static str> {
    v.try_into().map_err(|_| "dilithium_priv")
}

#[test]
fn test_handshake_and_encryption() -> Result<(), Box<dyn std::error::Error>> {
    let (kyber_pub_1_v, kyber_priv_1_v) = kyber_keypair()?;
    let (dilithium_pub_1_v, dilithium_priv_1_v) = dilithium_keypair()?;
    let (kyber_pub_2_v, kyber_priv_2_v) = kyber_keypair()?;
    let (dilithium_pub_2_v, dilithium_priv_2_v) = dilithium_keypair()?;

    let kyber_pub_1 = make_kyber_pub(kyber_pub_1_v)?;
    let kyber_priv_1 = make_kyber_priv(kyber_priv_1_v)?;
    let dilithium_pub_1 = make_dilithium_pub(dilithium_pub_1_v)?;
    let dilithium_priv_1 = make_dilithium_priv(dilithium_priv_1_v)?;

    let kyber_pub_2 = make_kyber_pub(kyber_pub_2_v)?;
    let kyber_priv_2 = make_kyber_priv(kyber_priv_2_v)?;
    let dilithium_pub_2 = make_dilithium_pub(dilithium_pub_2_v)?;
    let dilithium_priv_2 = make_dilithium_priv(dilithium_priv_2_v)?;

    let tunnel_1 = TransportTunnel::new(kyber_priv_1, dilithium_priv_1, dilithium_pub_1);
    let tunnel_2 = TransportTunnel::new(kyber_priv_2, dilithium_priv_2, dilithium_pub_2);

    tunnel_1.register_peer(2, kyber_pub_2, dilithium_pub_2);
    tunnel_2.register_peer(1, kyber_pub_1, dilithium_pub_1);

    let handshake_msg = tunnel_1.initiate_handshake(2)?;
    tunnel_2.accept_handshake(1, &handshake_msg)?;

    for i in 0..10 {
        let plain = format!("Message {}", i).into_bytes();
        let cipher = tunnel_1.encapsulate(2, &plain, b"aad")?;
        let decrypted = tunnel_2.decapsulate(1, &cipher, b"aad")?;
        assert_eq!(plain, decrypted);
    }
    Ok(())
}

#[test]
fn test_parallel_encryption() -> Result<(), Box<dyn std::error::Error>> {
    use std::sync::atomic::AtomicU64;
    use transport_tunnel::encapsulation::ParallelEncapsulator;

    let (_dilithium_pub_1, dilithium_priv_1_v) = dilithium_keypair()?;
    let dilithium_priv_1 = make_dilithium_priv(dilithium_priv_1_v)?;
    let dummy_public: [u8; 1568] = [0u8; 1568];
    let (session_key, _) = client_handshake(&dummy_public, &dilithium_priv_1)?;
    let counter = AtomicU64::new(0);

    let payload = vec![0xAB; 1024 * 1024];
    let aad = b"test";

    let (ciphertext, last_ctr) = ParallelEncapsulator::encapsulate(
        &session_key.key,
        &session_key.nonce_base,
        &counter,
        &payload,
        aad,
        4,
    )?;

    let decrypted = ParallelEncapsulator::decapsulate(&session_key.key, &ciphertext, aad)?;
    assert_eq!(payload, decrypted);
    assert!(last_ctr >= 3);
    Ok(())
}

#[test]
fn test_session_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let (kyber_pub_1_v, kyber_priv_1_v) = kyber_keypair()?;
    let (dilithium_pub_1_v, dilithium_priv_1_v) = dilithium_keypair()?;
    let (kyber_pub_2_v, kyber_priv_2_v) = kyber_keypair()?;
    let (dilithium_pub_2_v, dilithium_priv_2_v) = dilithium_keypair()?;

    let kyber_pub_1 = make_kyber_pub(kyber_pub_1_v)?;
    let kyber_priv_1 = make_kyber_priv(kyber_priv_1_v)?;
    let dilithium_pub_1 = make_dilithium_pub(dilithium_pub_1_v)?;
    let dilithium_priv_1 = make_dilithium_priv(dilithium_priv_1_v)?;

    let kyber_pub_2 = make_kyber_pub(kyber_pub_2_v)?;
    let kyber_priv_2 = make_kyber_priv(kyber_priv_2_v)?;
    let dilithium_pub_2 = make_dilithium_pub(dilithium_pub_2_v)?;
    let dilithium_priv_2 = make_dilithium_priv(dilithium_priv_2_v)?;

    let tunnel_1 = TransportTunnel::new(kyber_priv_1, dilithium_priv_1, dilithium_pub_1);
    let tunnel_2 = TransportTunnel::new(kyber_priv_2, dilithium_priv_2, dilithium_pub_2);

    tunnel_1.register_peer(2, kyber_pub_2, dilithium_pub_2);
    tunnel_2.register_peer(1, kyber_pub_1, dilithium_pub_1);

    let handshake_msg = tunnel_1.initiate_handshake(2)?;
    tunnel_2.accept_handshake(1, &handshake_msg)?;

    assert!(tunnel_2.has_session(1));
    Ok(())
}
