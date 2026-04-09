// Test liboqs ML-DSA-65 verify directly
#include <stdio.h>
#include <string.h>
#include <stdint.h>
#include <oqs/oqs.h>

int main() {
    OQS_init();
    
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    
    int ret = OQS_SIG_ml_dsa_65_keypair(public_key, secret_key);
    printf("Keypair: %d\n", ret);
    
    uint8_t message[] = "Test message";
    size_t message_len = strlen((char*)message);
    uint8_t signature[3309];
    size_t signature_len = 0;
    
    ret = OQS_SIG_ml_dsa_65_sign(signature, &signature_len, message, message_len, secret_key);
    printf("Sign: %d, len: %zu\n", ret, signature_len);
    
    // Verify with correct message
    ret = OQS_SIG_ml_dsa_65_verify(message, message_len, signature, signature_len, public_key);
    printf("Verify (correct): %d (expected 0)\n", ret);
    
    // Verify with wrong message
    uint8_t wrong_message[] = "Wrong message";
    ret = OQS_SIG_ml_dsa_65_verify(wrong_message, strlen((char*)wrong_message), signature, signature_len, public_key);
    printf("Verify (wrong): %d (expected non-zero)\n", ret);
    
    return 0;
}