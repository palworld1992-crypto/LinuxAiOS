extern "C" {
    pub fn spark_aes_gcm_encrypt(
        key: *const u8,
        plaintext: *const u8,
        plaintext_len: usize,
        aad: *const u8,
        aad_len: usize,
        out_len: *mut usize,
    ) -> *mut u8;
}