// C wrapper for OQS functions - global variable approach
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <oqs/oqs.h>

// Global to store last result
static int g_last_verify_result = 0;

void oqs_ml_dsa_65_verify_wrapper(
    const uint8_t *message,
    size_t message_len,
    const uint8_t *signature,
    size_t signature_len,
    const uint8_t *public_key
) {
    fprintf(stderr, "DEBUG: verify_wrapper void called\n");
    g_last_verify_result = OQS_SIG_ml_dsa_65_verify(message, message_len, signature, signature_len, public_key);
    fprintf(stderr, "DEBUG: stored result=%d\n", g_last_verify_result);
}

int oqs_ml_dsa_65_get_verify_result(void) {
    fprintf(stderr, "DEBUG: get_verify_result returning %d\n", g_last_verify_result);
    return g_last_verify_result;
}

void oqs_ml_dsa_65_verify_wrapper_out(
    const uint8_t *message,
    size_t message_len,
    const uint8_t *signature,
    size_t signature_len,
    const uint8_t *public_key,
    int *status) {
    *status = OQS_SIG_ml_dsa_65_verify(message, message_len, signature, signature_len, public_key);
}