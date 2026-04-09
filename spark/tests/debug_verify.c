// Debug Ada call to OQS verify
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <oqs/oqs.h>

// Ada procedure signature (from Ada's perspective)
extern void crypto_engine__dilithium_verify(
    const uint8_t *public_key,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *signature,
    size_t signature_len,
    int *status
);

// Direct OQS call for comparison
int direct_verify(const uint8_t *public_key, const uint8_t *message, size_t message_len,
                  const uint8_t *signature, size_t signature_len) {
    return OQS_SIG_ml_dsa_65_verify(message, message_len, signature, signature_len, public_key);
}

int main() {
    OQS_init();
    
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    uint8_t signature[3309];
    size_t signature_len = 0;
    int status = 0;
    
    // Generate keypair directly
    OQS_SIG_ml_dsa_65_keypair(public_key, secret_key);
    
    // Sign directly
    uint8_t message[] = "Test message from direct OQS";
    size_t message_len = strlen((char*)message);
    OQS_SIG_ml_dsa_65_sign(signature, &signature_len, message, message_len, secret_key);
    printf("Direct sign: signature_len=%zu\n", signature_len);
    
    // Verify correct message - direct OQS
    int direct_correct = direct_verify(public_key, message, message_len, signature, signature_len);
    printf("Direct verify (correct): %d\n", direct_correct);
    
    // Verify wrong message - direct OQS
    uint8_t wrong_msg[] = "WRONG message!!!";
    int direct_wrong = direct_verify(public_key, wrong_msg, strlen((char*)wrong_msg), signature, signature_len);
    printf("Direct verify (wrong): %d\n", direct_wrong);
    
    // Now test Ada FFI
    printf("\n--- Testing Ada FFI ---\n");
    
    // Verify correct message - Ada
    crypto_engine__dilithium_verify(public_key, message, message_len, signature, signature_len, &status);
    printf("Ada verify (correct): status=%d\n", status);
    
    // Verify wrong message - Ada
    crypto_engine__dilithium_verify(public_key, wrong_msg, strlen((char*)wrong_msg), signature, signature_len, &status);
    printf("Ada verify (wrong): status=%d\n", status);
    
    return 0;
}
