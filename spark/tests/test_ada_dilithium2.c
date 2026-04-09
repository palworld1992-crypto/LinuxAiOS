// Test Ada Dilithium FFI directly with detailed output
#include <stdio.h>
#include <string.h>
#include <stdint.h>

// Ada exported functions - note: out parameters are passed by reference
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

int main() {
    printf("Testing Ada Dilithium FFI with detailed output...\n");
    
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    int status = 0;
    
    // Initialize to non-zero to detect writes
    memset(public_key, 0xAA, sizeof(public_key));
    memset(secret_key, 0xBB, sizeof(secret_key));
    
    // Keypair
    crypto_engine__dilithium_keypair(public_key, secret_key, &status);
    printf("Keypair status: %d\n", status);
    
    // Check if keys were actually written
    int pubkey_changed = (public_key[0] != 0xAA || public_key[100] != 0xAA);
    int seckey_changed = (secret_key[0] != 0xBB || secret_key[100] != 0xBB);
    printf("Public key changed: %s\n", pubkey_changed ? "YES" : "NO");
    printf("Secret key changed: %s\n", seckey_changed ? "YES" : "NO");
    
    if (status != 0) {
        printf("Keypair failed!\n");
        return 1;
    }
    
    // Sign
    uint8_t message[] = "Hello, Ada Dilithium!";
    size_t message_len = strlen((char*)message);
    uint8_t signature[3309];
    size_t signature_len = 0;
    
    // Initialize to detect writes
    memset(signature, 0xCC, sizeof(signature));
    signature_len = 0;  // Initialize to 0
    
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
    
    // Check if signature was actually written
    int sig_changed = (signature[0] != 0xCC || signature[10] != 0xCC);
    printf("Signature buffer changed: %s\n", sig_changed ? "YES" : "NO");
    
    if (signature_len > 0) {
        printf("First 32 bytes of signature:\n");
        for (int i = 0; i < 32 && i < signature_len; i++) {
            printf("%02x ", signature[i]);
            if ((i + 1) % 16 == 0) printf("\n");
        }
    }
    
    return status;
}