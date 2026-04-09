#include <linux/bpf.h>
#include <linux/types.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

/* * FIX: Khai báo thủ công nếu header hệ thống của bạn cũ 
 * giúp tránh lỗi 'undeclared function' trong Zig build 
 */
extern long bpf_msg_load_bytes(struct sk_msg_md *ctx, __u32 offset, void *to, __u32 len);

#define PEER_ID_SIZE 8

// Map: key = destination peer ID (8 bytes), value = socket fd
struct {
    __uint(type, BPF_MAP_TYPE_SOCKMAP);
    __uint(max_entries, 1024);
    __uint(key_size, PEER_ID_SIZE);
    __uint(value_size, sizeof(int));
} sock_map SEC(".maps");

SEC("sk_msg")
int msg_redirect(struct sk_msg_md *msg) {
    // SỬA: Dùng __u64 thay cho uint64_t để tránh phụ thuộc stdint.h
    __u64 dest_id = 0;

    // Kéo dữ liệu vào tuyến tính hóa để có thể đọc
    if (bpf_msg_pull_data(msg, 0, PEER_ID_SIZE, 0) < 0) {
        return SK_DROP;
    }

    // Đọc 8 bytes ID đích
    if (bpf_msg_load_bytes(msg, 0, &dest_id, PEER_ID_SIZE) < 0) {
        return SK_DROP;
    }

    /* * SỬA LOGIC QUAN TRỌNG:
     * bpf_msg_redirect_map không cần bạn phải tự lookup trước.
     * Bạn chỉ cần truyền KEY (dest_id) vào, Kernel sẽ tự tìm socket tương ứng trong map.
     * Tham số thứ 3 phải là giá trị (u32/u64), không phải con trỏ int*.
     */
    return bpf_msg_redirect_map(msg, &sock_map, dest_id, BPF_F_INGRESS);
}

char _license[] SEC("license") = "GPL";