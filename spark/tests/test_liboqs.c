// Test liboqs ML-DSA-65
#include <stdio.h>
#include <string.h>
#include <oqs/oqs.h>

int main() {
    printf("OQS version: %s\n", OQS_version());
    
    // Initialize
    OQS_init();
    
    // Check if ML-DSA-65 is enabled
    if (OQS_SIG_alg_is_enabled("ML-DSA-65") == OQS_SUCCESS) {
        printf("ML-DSA-65 is enabled\n");
    } else {
        printf("ML-DSA-65 is NOT enabled\n");
    }
    
    // Generate keypair
    uint8_t public_key[1952];
    uint8_t secret_key[4032];
    
    int ret = OQS_SIG_ml_dsa_65_keypair(public_key, secret_key);
    printf("Keypair result: %d\n", ret);
    
    if (ret == OQS_SUCCESS) {
        // Sign
        uint8_t message[] = "Hello, liboqs!";
        size_t message_len = strlen((char*)message);
        uint8_t signature[3309];
        size_t signature_len = 0;
        
        ret = OQS_SIG_ml_dsa_65_sign(signature, &signature_len, message, message_len, secret_key);
        printf("Sign result: %d, signature_len: %zu\n", ret, signature_len);
        
        if (ret == OQS_SUCCESS) {
            // Verify
            ret = OQS_SIG_ml_dsa_65_verify(message, message_len, signature, signature_len, public_key);
            printf("Verify result: %d\n", ret);
        }
    }
    
    return 0;
}