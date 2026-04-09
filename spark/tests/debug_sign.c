// Debug helper to check what Ada is passing
#include <stdio.h>
#include <stdint.h>
#include <oqs/oqs.h>

// Print OQS_SIG_ml_dsa_65_sign call
int debug_ml_dsa_65_sign(
    uint8_t *signature,
    size_t *signature_len,
    const uint8_t *message,
    size_t message_len,
    const uint8_t *secret_key
) {
    printf("DEBUG sign: signature=%p, signature_len=%p (*signature_len=%zu), message_len=%zu\n",
           (void*)signature, (void*)signature_len, signature_len ? *signature_len : 0, message_len);
    
    int result = OQS_SIG_ml_dsa_65_sign(signature, signature_len, message, message_len, secret_key);
    
    printf("DEBUG sign result=%d, *signature_len=%zu\n", result, signature_len ? *signature_len : 0);
    return result;
}