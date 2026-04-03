#!/bin/sh
# Dừng script nếu có lỗi nghiêm trọng
set -e

# Chuyển đến thư mục chứa script
cd "$(dirname "$0")"

echo "--- 🛠️  Đang kiểm tra môi trường build cho LinusAiOS ---"

# 1. BIÊN DỊCH PHẦN SPARK/ADA
if command -v gprbuild >/dev/null 2>&1; then
    echo "[1/2] Tìm thấy gprbuild, đang biên dịch mã nguồn SPARK..."
    # Viết liền -P với tên file để tránh lỗi mất tham số
    gprbuild -Pspark.gpr -p
else
    echo "[1/2] ⚠️ Không tìm thấy gprbuild. Kiểm tra thư viện tĩnh có sẵn..."
    # Kiểm tra trong thư mục lib (thường nằm trong spark/lib)
    if [ ! -f "./lib/libscc.a" ] && [ ! -f "./spark/lib/libscc.a" ]; then
        echo "❌ Lỗi: Không tìm thấy libscc.a! Bạn cần gprbuild để tạo file này."
        exit 1
    fi
fi

# 2. BIÊN DỊCH PHẦN RUST (MODULE SCC & APP CHÍNH)
echo "[2/2] Đang biên dịch phần Rust & Liên kết với Kyber/Dilithium..."

# Lưu lại đường dẫn hiện tại để quay lại nếu cần
CURRENT_DIR=$(pwd)

# Tìm thư mục rust dựa trên vị trí file build.sh
if [ -d "./rust" ]; then
    cd ./rust
elif [ -d "../rust" ]; then
    cd ../rust
else
    echo "❌ Lỗi: Không tìm thấy thư mục rust!"
    exit 1
fi

# Chạy cargo build
cargo build --release

# Quay lại thư mục ban đầu
cd "$CURRENT_DIR"

echo "--- ✅ Hoàn tất quá trình cập nhật cho ToHung! ---"