import time
import os
import requests
from dotenv import load_dotenv
from collections import deque
from threading import Lock

# Tải cấu hình từ .env
load_dotenv()

# Lấy các biến từ file cấu hình của bạn
API_KEY = os.getenv("OPENROUTER_API_KEY")
BASE_URL = os.getenv("OPENROUTER_BASE_URL", "https://openrouter.ai/api/v1")
DEBOUNCE = int(os.getenv("OPENCODE_API_DEBOUNCE", 10000)) / 1000  # 10 giây
RETRY_DELAY = int(os.getenv("OPENCODE_RETRY_DELAY", 20000)) / 1000 # 20 giây
MIN_REQUEST_INTERVAL = int(os.getenv("OPENCODE_MIN_REQUEST_INTERVAL", 15000)) / 1000  # 15 giây giữa các request

# Queue và rate limiting
request_queue = deque()
queue_lock = Lock()
last_request_time = 0
consecutive_failures = 0
MAX_CONSECUTIVE_FAILURES = 3

def _wait_for_rate_limit():
    """Đợi nếu request trước đó quá gần"""
    global last_request_time
    now = time.time()
    elapsed = now - last_request_time
    if elapsed < MIN_REQUEST_INTERVAL:
        wait_time = MIN_REQUEST_INTERVAL - elapsed
        print(f"[*] Rate limit: đợi thêm {wait_time:.1f}s trước khi gửi...")
        time.sleep(wait_time)
    last_request_time = time.time()

def _calculate_adaptive_delay():
    """Tính độ trễ thích ứng dựa trên số lỗi liên tiếp"""
    global consecutive_failures
    if consecutive_failures > 0:
        # Tăng delay theo cấp số nhân khi có lỗi liên tiếp
        adaptive = RETRY_DELAY * (2 ** (consecutive_failures - 1))
        adaptive = min(adaptive, 120)  # Max 2 phút
        print(f"[*] Adaptive delay: {adaptive:.1f}s (do {consecutive_failures} lỗi liên tiếp)")
        return adaptive
    return 0

def run_supervisor_task(prompt):
    global consecutive_failures, last_request_time
    print(f"\n[HỆ THỐNG] Đã nhận lệnh: '{prompt}'")
    
    # Bước 1: Debounce - chờ để tránh gửi quá nhanh
    print(f"[*] Đang chờ {DEBOUNCE}s (Debounce) để tránh gửi quá nhanh...")
    time.sleep(DEBOUNCE)

    # Bước 2: Adaptive delay nếu có lỗi trước đó
    adaptive_delay = _calculate_adaptive_delay()
    if adaptive_delay > 0:
        time.sleep(adaptive_delay)

    # Bước 3: Rate limiting
    _wait_for_rate_limit()

    headers = {
        "Authorization": f"Bearer {API_KEY}",
        "Content-Type": "application/json"
    }
    
    payload = {
        "model": "qwen/qwen-3.6-plus:free",
        "messages": [{"role": "user", "content": prompt}]
    }

    attempt = 1
    while True:
        print(f"\n[Lần thử {attempt}] Đang kết nối tới Server OpenRouter...")
        try:
            response = requests.post(f"{BASE_URL}/chat/completions", headers=headers, json=payload, timeout=60)
            data = response.json()

            # Kiểm tra lỗi 429/502/503
            if response.status_code in [429, 502, 503] or "error" in data:
                consecutive_failures += 1
                error_info = data.get('error', {}).get('message', 'Server Busy')
                print(f"⚠️ CẢNH BÁO: Provider báo lỗi (status: {response.status_code}).")
                print(f"   - Chi tiết: {error_info}")
                print(f"   - Số lỗi liên tiếp: {consecutive_failures}")
                
                # Nếu gặp 502, tự động gửi "ok" để kết nối lại
                if response.status_code == 502:
                    print(f"🔄 LỖI 502: Tự động gửi 'ok' để khôi phục kết nối...")
                    reconnect_payload = {
                        "model": "qwen/qwen-3.6-plus:free",
                        "messages": [{"role": "user", "content": "ok"}]
                    }
                    time.sleep(3)  # Đợi ngắn trước khi reconnect
                    try:
                        reconnect_response = requests.post(
                            f"{BASE_URL}/chat/completions",
                            headers=headers,
                            json=reconnect_payload,
                            timeout=30
                        )
                        if reconnect_response.status_code == 200:
                            print("✅ Kết nối lại thành công! Đang gửi lại request chính...")
                            consecutive_failures = 0  # Reset counter
                            attempt = 1  # Reset attempt
                            continue  # Gửi lại request gốc
                        else:
                            print(f"⚠️ Reconnect thất bại (status: {reconnect_response.status_code})")
                    except Exception as reconnect_err:
                        print(f"⚠️ Lỗi khi kết nối lại: {reconnect_err}")
                
                # Adaptive retry delay
                retry_wait = RETRY_DELAY * (2 ** (consecutive_failures - 1))
                retry_wait = min(retry_wait, 120)
                print(f"   - Đợi {retry_wait:.1f}s trước khi thử lại...")
                time.sleep(retry_wait)
                attempt += 1
                continue 

            # Thành công - reset counter
            consecutive_failures = 0
            print(f"✅ THÀNH CÔNG: Model đã phản hồi tại lần thử thứ {attempt}!")
            return data['choices'][0]['message']['content']

        except requests.exceptions.Timeout:
            consecutive_failures += 1
            print(f"⏱️ TIMEOUT: Request vượt quá 60s")
            retry_wait = RETRY_DELAY * (2 ** (consecutive_failures - 1))
            retry_wait = min(retry_wait, 120)
            print(f"   - Đợi {retry_wait:.1f}s trước khi thử lại...")
            time.sleep(retry_wait)
            attempt += 1
            
        except Exception as e:
            consecutive_failures += 1
            print(f"❌ LỖI KẾT NỐI: {e}")
            retry_wait = RETRY_DELAY * (2 ** (consecutive_failures - 1))
            retry_wait = min(retry_wait, 120)
            print(f"   - Đợi {retry_wait:.1f}s trước khi thử lại...")
            time.sleep(retry_wait)
            attempt += 1

if __name__ == "__main__":
    print("=== TRÌNH GIÁM SÁT GIAO TIẾP AI ĐANG HOẠT ĐỘNG ===")
    ket_qua = run_supervisor_task("làm đi")
    print("-" * 50)
    print("KẾT QUẢ CUỐI CÙNG:")
    print(ket_qua)
