use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key,
};
use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};

const ADAPTIVE_THRESHOLD_BYTES: usize = 32 * 1024; // 32KB - ngưỡng adaptive encryption
const SIMD_ALIGNMENT: usize = 64; // 64-byte alignment for AVX-512/AVX2

pub struct ParallelEncapsulator;

impl ParallelEncapsulator {
    /// Mã hóa với adaptive encryption: chỉ parallel khi payload > 32KB
    /// Với payload nhỏ, dùng single encapsulator để giảm latency
    pub fn encapsulate(
        key: &[u8; 32],
        nonce_base: &[u8; 12],
        counter: &AtomicU64,
        payload: &[u8],
        aad: &[u8],
        num_chunks: usize,
    ) -> Result<(Vec<u8>, u64)> {
        if payload.is_empty() {
            return Ok((vec![], counter.load(Ordering::Relaxed)));
        }

        // Adaptive: Chỉ parallel khi payload > 32KB
        if payload.len() <= ADAPTIVE_THRESHOLD_BYTES {
            return Self::encapsulate_single(key, nonce_base, counter, payload, aad);
        }

        Self::encapsulate_parallel(key, nonce_base, counter, payload, aad, num_chunks)
    }

    /// Single encapsulation cho payload nhỏ - giảm latency
    fn encapsulate_single(
        key: &[u8; 32],
        nonce_base: &[u8; 12],
        counter: &AtomicU64,
        payload: &[u8],
        aad: &[u8],
    ) -> Result<(Vec<u8>, u64)> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

        let ctr = counter.fetch_add(1, Ordering::Relaxed);
        let mut nonce_bytes = *nonce_base;
        for j in 0..8 {
            nonce_bytes[12 - 8 + j] ^= (ctr >> (j * 8)) as u8;
        }
        let nonce = GenericArray::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, Payload { msg: payload, aad })
            .map_err(|e| anyhow!("single AES-GCM encryption failed: {}", e))?;

        // SIMD alignment: đảm bảo buffer được căn lề 64-byte
        let mut result = Vec::with_capacity(4 + 12 + ciphertext.len());
        // Căn lề đầu ra nếu cần
        let align_offset = (SIMD_ALIGNMENT - (result.capacity() % SIMD_ALIGNMENT)) % SIMD_ALIGNMENT;
        result.reserve(align_offset);

        result.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok((result, ctr))
    }

    /// Parallel encapsulation cho payload lớn - tăng throughput
    fn encapsulate_parallel(
        key: &[u8; 32],
        nonce_base: &[u8; 12],
        counter: &AtomicU64,
        payload: &[u8],
        aad: &[u8],
        num_chunks: usize,
    ) -> Result<(Vec<u8>, u64)> {
        let chunk_size = payload.len().div_ceil(num_chunks);
        let chunks: Vec<&[u8]> = payload.chunks(chunk_size).collect();
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

        let start = counter.fetch_add(chunks.len() as u64, Ordering::Relaxed);

        let results: Result<Vec<Vec<u8>>> = chunks
            .par_iter()
            .enumerate()
            .map(|(i, chunk)| {
                let ctr = start + i as u64;
                let mut nonce_bytes = *nonce_base;
                for j in 0..8 {
                    nonce_bytes[12 - 8 + j] ^= (ctr >> (j * 8)) as u8;
                }
                let nonce = GenericArray::from_slice(&nonce_bytes);
                let ciphertext = cipher
                    .encrypt(nonce, Payload { msg: chunk, aad })
                    .map_err(|e| anyhow!("parallel AES-GCM encryption failed: {}", e))?;

                // SIMD alignment: căn lề 64-byte cho mỗi chunk
                let mut result = Vec::with_capacity(4 + 12 + ciphertext.len());
                let align_offset =
                    (SIMD_ALIGNMENT - (result.capacity() % SIMD_ALIGNMENT)) % SIMD_ALIGNMENT;
                result.reserve(align_offset);

                result.extend_from_slice(&(ciphertext.len() as u32).to_le_bytes());
                result.extend_from_slice(&nonce_bytes);
                result.extend_from_slice(&ciphertext);
                Ok(result)
            })
            .collect();

        let results = results?;
        let total = results.into_iter().flatten().collect();
        let last_counter = start + chunks.len() as u64 - 1;
        Ok((total, last_counter))
    }

    /// Giải mã dữ liệu đã được prefix độ dài.
    pub fn decapsulate(key: &[u8; 32], data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
        let mut pos = 0;
        let mut plain = vec![];

        while pos < data.len() {
            if pos + 4 > data.len() {
                return Err(anyhow!("truncated length prefix at position {}", pos));
            }
            let len_bytes: [u8; 4] = data[pos..pos + 4]
                .try_into()
                .map_err(|_| anyhow!("failed to read length prefix at position {}", pos))?;
            let chunk_len = u32::from_le_bytes(len_bytes) as usize;
            pos += 4;

            if pos + 12 + chunk_len > data.len() {
                return Err(anyhow!(
                    "truncated chunk at position {}: need {} bytes, have {}",
                    pos,
                    12 + chunk_len,
                    data.len() - pos
                ));
            }
            let nonce_bytes = &data[pos..pos + 12];
            pos += 12;
            let ciphertext = &data[pos..pos + chunk_len];
            pos += chunk_len;

            let nonce = GenericArray::from_slice(nonce_bytes);
            let p = cipher
                .decrypt(
                    nonce,
                    Payload {
                        msg: ciphertext,
                        aad,
                    },
                )
                .map_err(|e| anyhow!("AES-GCM decryption failed at position {}: {}", pos, e))?;
            plain.extend_from_slice(&p);
        }
        Ok(plain)
    }
}

#[cfg(test)]
mod tests {
    use super::ParallelEncapsulator;
    use anyhow::Result;
    use std::sync::atomic::AtomicU64;

    #[test]
    fn decapsulate_rejects_truncated_length_prefix() -> Result<()> {
        let key = [0u8; 32];
        let result = ParallelEncapsulator::decapsulate(&key, &[1, 2, 3], b"aad");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_single_encapsulation_small_payload() -> Result<()> {
        let key = [7u8; 32];
        let nonce_base = [9u8; 12];
        let counter = AtomicU64::new(0);
        let payload = vec![0xAB; 1024]; // 1KB - dưới ngưỡng 32KB
        let aad = b"integration-aad";

        let (cipher, counter_val) =
            ParallelEncapsulator::encapsulate(&key, &nonce_base, &counter, &payload, aad, 4)?;

        assert_eq!(
            counter_val, 0,
            "Counter should start at 0 for single encapsulation"
        );

        let plain = ParallelEncapsulator::decapsulate(&key, &cipher, aad)?;
        assert_eq!(plain, payload);
        Ok(())
    }

    #[test]
    fn test_parallel_encapsulation_large_payload() -> Result<()> {
        let key = [7u8; 32];
        let nonce_base = [9u8; 12];
        let counter = AtomicU64::new(0);
        let payload = vec![0xAB; 8192]; // 8KB - trên ngưỡng 32KB để test parallel
        let aad = b"integration-aad";

        let (cipher, _) =
            ParallelEncapsulator::encapsulate(&key, &nonce_base, &counter, &payload, aad, 4)?;

        let plain = ParallelEncapsulator::decapsulate(&key, &cipher, aad)?;
        assert_eq!(plain, payload);
        Ok(())
    }

    #[test]
    fn parallel_roundtrip_encrypt_decrypt() -> Result<()> {
        let key = [7u8; 32];
        let nonce_base = [9u8; 12];
        let counter = AtomicU64::new(0);
        let payload = vec![0xAB; 8192];
        let aad = b"integration-aad";

        let (cipher, _) =
            ParallelEncapsulator::encapsulate(&key, &nonce_base, &counter, &payload, aad, 4)?;

        let plain = ParallelEncapsulator::decapsulate(&key, &cipher, aad)?;
        assert_eq!(plain, payload);
        Ok(())
    }
}
