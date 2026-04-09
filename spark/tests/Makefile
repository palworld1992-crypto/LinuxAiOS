# Makefile for SPARK/Ada crypto tests

CC = gcc
CFLAGS = -Wall -Wextra -std=c11
OQS_CFLAGS = $(CFLAGS) -I/usr/local/include
OQS_LDFLAGS = -loqs -lcrypto

# Library paths
SPARK_LIB = ../lib
LDFLAGS = -L$(SPARK_LIB) -lscc -loqs_wrapper -loqs -lgnat -lgnarl -lcrypto -lssl -lpthread -lm -ldl -Wl,-rpath,$(SPARK_LIB)

# Test targets
TESTS_LIBOQS = test_liboqs test_oqs_verify_direct test_oqs_wrong_msg
TESTS_ADA = test_ada_dilithium test_ada_dilithium2 test_ada_dilithium_verify
TESTS_DEBUG = debug_sign debug_verify
TESTS_ALL = $(TESTS_LIBOQS) $(TESTS_ADA)

.PHONY: all clean run_all

all: $(TESTS_ALL)

# Tests for liboqs (direct)
test_liboqs: test_liboqs.c
	$(CC) $(OQS_CFLAGS) -o $@ $< $(OQS_LDFLAGS)

test_oqs_verify_direct: test_oqs_verify_direct.c
	$(CC) $(OQS_CFLAGS) -o $@ $< $(OQS_LDFLAGS)

test_oqs_wrong_msg: test_oqs_wrong_msg.c
	$(CC) $(OQS_CFLAGS) -o $@ $< $(OQS_LDFLAGS)

# Tests for Ada FFI
test_ada_dilithium: test_ada_dilithium.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

test_ada_dilithium2: test_ada_dilithium2.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

test_ada_dilithium_verify: test_ada_dilithium_verify.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

# Debug tests
debug_sign: debug_sign.c
	$(CC) $(OQS_CFLAGS) -o $@ $< $(OQS_LDFLAGS)

debug_verify: debug_verify.c
	$(CC) $(CFLAGS) -o $@ $< $(LDFLAGS)

# Run all tests
run_all: all
	@echo "=== Running liboqs tests ==="
	@LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_liboqs
	@echo ""
	@LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_oqs_verify_direct
	@echo ""
	@LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_oqs_wrong_msg
	@echo ""
	@echo "=== Running Ada FFI tests ==="
	@LD_PRELOAD=$(SPARK_LIB)/liboqs_wrapper.so LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_ada_dilithium
	@echo ""
	@LD_PRELOAD=$(SPARK_LIB)/liboqs_wrapper.so LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_ada_dilithium2
	@echo ""
	@LD_PRELOAD=$(SPARK_LIB)/liboqs_wrapper.so LD_LIBRARY_PATH=$(SPARK_LIB):$$LD_LIBRARY_PATH ./test_ada_dilithium_verify
	@echo ""
	@echo "=== All tests completed ==="

clean:
	rm -f $(TESTS_ALL) *.o