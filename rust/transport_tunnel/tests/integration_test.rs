#[cfg(test)]
mod tests {
    use scc::crypto::{dilithium_keypair, kyber_keypair};
    use transport_tunnel::{client_handshake, TransportTunnel};

    #[test]
    #[ignore = "Depends on Ada/OQS crypto runtime; unstable in CI/dev env"]
    fn test_handshake_and_encryption() -> Result<(), Box<dyn std::error::Error>> {
        let (_kyber_pub_1, kyber_priv_1) = kyber_keypair()?;
        let (dilithium_pub_1, dilithium_priv_1) = dilithium_keypair()?;
        let (kyber_pub_2, _kyber_priv_2) = kyber_keypair()?;
        let (dilithium_pub_2, _dilithium_priv_2) = dilithium_keypair()?;

        let kyber_priv_1: [u8; 2400] = kyber_priv_1.try_into().map_err(|_| "kyber_priv_1")?;
        let dilithium_priv_1: [u8; 4032] = dilithium_priv_1
            .try_into()
            .map_err(|_| "dilithium_priv_1")?;
        let dilithium_pub_1: [u8; 1952] =
            dilithium_pub_1.try_into().map_err(|_| "dilithium_pub_1")?;
        let kyber_pub_2_arr: [u8; 1568] = kyber_pub_2.try_into().map_err(|_| "kyber_pub_2")?;
        let dilithium_pub_2_arr: [u8; 1952] =
            dilithium_pub_2.try_into().map_err(|_| "dilithium_pub_2")?;

        let tunnel_1 = TransportTunnel::new(kyber_priv_1, dilithium_priv_1, dilithium_pub_1);

        let (kyber_pub_1_arr, kyber_priv_2_arr) = kyber_keypair()?;
        let (dilithium_pub_1_arr, dilithium_priv_2_arr) = dilithium_keypair()?;

        let kyber_priv_2: [u8; 2400] = kyber_priv_2_arr.try_into().map_err(|_| "kyber_priv_2")?;
        let dilithium_priv_2: [u8; 4032] = dilithium_priv_2_arr
            .try_into()
            .map_err(|_| "dilithium_priv_2")?;
        let dilithium_pub_2: [u8; 1952] = dilithium_pub_1_arr
            .try_into()
            .map_err(|_| "dilithium_pub_2_new")?;
        let kyber_pub_1: [u8; 1568] = kyber_pub_1_arr.try_into().map_err(|_| "kyber_pub_1")?;
        let dilithium_pub_1: [u8; 1952] =
            dilithium_pub_2.try_into().map_err(|_| "dilithium_pub_1")?;

        let tunnel_2 = TransportTunnel::new(kyber_priv_2, dilithium_priv_2, dilithium_pub_2);

        tunnel_1.register_peer(2, kyber_pub_2_arr, dilithium_pub_2_arr);
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
    #[ignore = "Depends on Ada/OQS crypto runtime; unstable in CI/dev env"]
    fn test_parallel_encryption() -> Result<(), Box<dyn std::error::Error>> {
        use std::sync::atomic::AtomicU64;
        use transport_tunnel::encapsulation::ParallelEncapsulator;

        let (_dilithium_pub_1, dilithium_priv_1) = dilithium_keypair()?;
        let dilithium_priv_1: [u8; 4032] = dilithium_priv_1
            .try_into()
            .map_err(|_| "dilithium_priv_1")?;
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

        let decrypted = ParallelEncapsulator::decapsulate(&session_key.key, &ciphertext, aad)
            .ok_or("decryption failed")?;
        assert_eq!(payload, decrypted);
        assert!(last_ctr >= 3);
        Ok(())
    }

    #[test]
    #[ignore = "Depends on Ada/OQS crypto runtime; unstable in CI/dev env"]
    fn test_session_timeout() -> Result<(), Box<dyn std::error::Error>> {
        let (_kyber_pub_1, kyber_priv_1) = kyber_keypair()?;
        let (dilithium_pub_1, dilithium_priv_1) = dilithium_keypair()?;
        let (kyber_pub_2, _kyber_priv_2) = kyber_keypair()?;
        let (dilithium_pub_2, _dilithium_priv_2) = dilithium_keypair()?;

        let kyber_priv_1: [u8; 2400] = kyber_priv_1.try_into().map_err(|_| "kyber_priv_1")?;
        let dilithium_priv_1: [u8; 4032] = dilithium_priv_1
            .try_into()
            .map_err(|_| "dilithium_priv_1")?;
        let dilithium_pub_1: [u8; 1952] =
            dilithium_pub_1.try_into().map_err(|_| "dilithium_pub_1")?;
        let kyber_pub_2_arr: [u8; 1568] = kyber_pub_2.try_into().map_err(|_| "kyber_pub_2")?;
        let dilithium_pub_2_arr: [u8; 1952] =
            dilithium_pub_2.try_into().map_err(|_| "dilithium_pub_2")?;

        let tunnel_1 = TransportTunnel::new(kyber_priv_1, dilithium_priv_1, dilithium_pub_1);

        let (kyber_pub_1_arr, kyber_priv_2_arr) = kyber_keypair()?;
        let (dilithium_pub_1_arr, dilithium_priv_2_arr) = dilithium_keypair()?;

        let kyber_priv_2: [u8; 2400] = kyber_priv_2_arr.try_into().map_err(|_| "kyber_priv_2")?;
        let dilithium_priv_2: [u8; 4032] = dilithium_priv_2_arr
            .try_into()
            .map_err(|_| "dilithium_priv_2")?;
        let dilithium_pub_2: [u8; 1952] = dilithium_pub_1_arr
            .try_into()
            .map_err(|_| "dilithium_pub_2_new")?;
        let kyber_pub_1: [u8; 1568] = kyber_pub_1_arr.try_into().map_err(|_| "kyber_pub_1")?;
        let dilithium_pub_1: [u8; 1952] =
            dilithium_pub_2.try_into().map_err(|_| "dilithium_pub_1")?;

        let tunnel_2 = TransportTunnel::new(kyber_priv_2, dilithium_priv_2, dilithium_pub_2);

        tunnel_1.register_peer(2, kyber_pub_2_arr, dilithium_pub_2_arr);
        tunnel_2.register_peer(1, kyber_pub_1, dilithium_pub_1);

        let handshake_msg = tunnel_1.initiate_handshake(2)?;
        tunnel_2.accept_handshake(1, &handshake_msg)?;

        assert!(tunnel_2.has_session(1));
        Ok(())
    }
}
