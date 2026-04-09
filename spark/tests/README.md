# SPARK/Ada Crypto Tests

Thư mục này chứa các file test C để kiểm tra chức năng crypto từ Ada/SPARK backend.

## Cấu trúc

```
spark/tests/
├── README.md                    # File này
├── Makefile                     # Build script
│
│── # Tests liboqs trực tiếp
├── test_liboqs.c                # Test liboqs ML-DSA-65 trực tiếp
├── test_oqs_verify_direct.c     # Test liboqs verify trực tiếp
├── test_oqs_wrong_msg.c         # Test liboqs với message sai
│
│── # Tests Ada FFI
├── test_ada_dilithium.c         # Test Ada Dilithium FFI
├── test_ada_dilithium2.c        # Test Ada Dilithium FFI với detailed output
├── test_ada_dilithium_verify.c  # Test Ada Dilithium verify FFI
│
│── # Debug helpers
├── debug_sign.c                 # Debug helper cho sign function
└── debug_verify.c               # Debug helper cho verify function
```

## Yêu cầu

- GCC
- liboqs (đã cài đặt tại `/usr/local` hoặc `/usr`)
- libscc.so (build từ `spark/`)
- liboqs_wrapper.so (build từ `spark/crypto/oqs_wrapper.c`)

## Build

### Build libscc.so và liboqs_wrapper.so

```bash
cd spark
/usr/libexec/spark/bin/gprbuild -P spark.gpr -p -f -cargs -fPIC
```

### Build test files

```bash
cd spark/tests
make all
```

Hoặc build test cụ thể:

```bash
make test_liboqs
make test_ada_dilithium
make test_ada_dilithium_verify
```

## Run tests

```bash
# Chạy tất cả tests
make run_all

# Hoặc chạy test cụ thể
./test_liboqs
./test_ada_dilithium
```

## Lưu ý

- Tests yêu cầu `LD_LIBRARY_PATH` hoặc `LD_PRELOAD` để tìm `libscc.so` và `liboqs_wrapper.so`
- `make run_all` tự động set `LD_LIBRARY_PATH`
- Các test `test_ada_*` kiểm tra FFI giữa C và Ada
- Các test `test_oqs_*` kiểm tra liboqs trực tiếp

## Kết quả mong đợi

```
test_liboqs:
  OQS version: x.x.x
  ML-DSA-65 is enabled
  Keypair result: 0
  Sign result: 0, signature_len: 3309
  Verify result: 0

test_ada_dilithium_verify:
  Keypair status: 0
  Sign status: 0, signature_len: 3309
  Verify (correct message): status=0 (expected 0)
  Verify (wrong message): status=-1 (expected non-zero)
```