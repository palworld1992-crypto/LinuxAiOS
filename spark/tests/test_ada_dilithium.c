// Test Ada Dilithium FFI directly
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

extern void crypto_engine__dilithium_verify(
    const uint8_t *public_key,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *signature,
    size_t signature_len,
    int *status
);

int main() {
    printf("Testing Ada Dilithium FFI...\n");
    
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    int status = 0;
    
    // Keypair
    crypto_engine__dilithium_keypair(public_key, secret_key, &status);
    printf("Keypair status: %d\n", status);
    
    if (status != 0) {
        printf("Keypair failed!\n");
        return 1;
    }
    
    // Sign
    uint8_t message[] = "Hello, Ada Dilithium!";
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
    
    if (status != 0) {
        printf("Sign failed!\n");
        return 1;
    }
    
    // Verify
    crypto_engine__dilithium_verify(
        public_key,
        message,
        message_len,
        signature,
        signature_len,
        &status
    );
    printf("Verify status: %d\n", status);
    
    return status;
}