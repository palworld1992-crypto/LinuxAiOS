// Test Ada Dilithium Verify
#include <stdio.h>
#include <string.h>
#include <stdint.h>

extern void crypto_engine__dilithium_keypair(
    uint8_t *public_key,
    uint8_t *secret_key,
    int *status
);

extern void crypto_engine__dilithium_sign(
    const uint8_t *secret_key,
    const uint8_t *message,
    size_t message_len,
    uint8_t *signature_buf,
    size_t signature_buf_size,
    size_t *signature_len,
    int *status
);

extern void crypto_engine__dilithium_verify(
    const uint8_t *public_key,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *signature,
    size_t signature_len,
    int *status
);

int main() {
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    int status = 0;
    
    // Keypair
    crypto_engine__dilithium_keypair(public_key, secret_key, &status);
    printf("Keypair status: %d\n", status);
    
    // Sign
    uint8_t message[] = "Test message for verification";
    size_t message_len = strlen((char*)message);
    uint8_t signature[3309];
    size_t signature_len = 0;
    
    crypto_engine__dilithium_sign(
        secret_key,
        message,
        message_len,
        signature,
        sizeof(signature),
        &signature_len,
        &status
    );
    printf("Sign status: %d, signature_len: %zu\n", status, signature_len);
    
    // Verify with correct message
    crypto_engine__dilithium_verify(
        public_key,
        message,
        message_len,
        signature,
        signature_len,
        &status
    );
    printf("Verify (correct message): status=%d (expected 0)\n", status);
    
    // Verify with wrong message
    uint8_t wrong_message[] = "Wrong message";
    crypto_engine__dilithium_verify(
        public_key,
        wrong_message,
        strlen((char*)wrong_message),
        signature,
        signature_len,
        &status
    );
    printf("Verify (wrong message): status=%d (expected non-zero)\n", status);
    
    return 0;
}