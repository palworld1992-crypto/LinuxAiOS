// eBPF program for cold page detection - AIOS Enterprise Edition
// Attaches to tracepoint/exceptions/page_fault_user
// Records timestamp of each user page fault.

#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <linux/types.h>

/* * Enterprise Fix 1: Định nghĩa cấu trúc trace_entry cơ bản.
 * Đây là 8 bytes đầu tiên của mọi tracepoint trong Linux Kernel.
 */
struct trace_entry {
    unsigned short type;
    unsigned char flags;
    unsigned char preempt_count;
    int pid;
};

/* * Enterprise Fix 2: Định nghĩa cấu trúc tracepoint cụ thể cho x86.
 * Việc tự định nghĩa giúp code độc lập với các header hệ thống hay bị lỗi trên WSL2.
 */
struct trace_event_raw_x86_exceptions {
    struct trace_entry ent; // 8 bytes đầu
    __u64 address;          // Offset đúng để đọc địa chỉ gây page fault
    __u64 ip;               // Instruction pointer
    __u64 error_code;       // Mã lỗi CPU
};

// Sử dụng attribute packed để đảm bảo Zig 0.15.2 đọc dữ liệu không bị sai lệch padding
struct key_t {
    __u32 pid;
    __u64 vaddr;
} __attribute__((packed));

struct value_t {
    __u64 timestamp_ns;
} __attribute__((packed));

// LRU Hash Map: Tối ưu cho hiệu năng hệ thống AIOS
struct {
    __uint(type, BPF_MAP_TYPE_LRU_HASH);
    __uint(max_entries, 1024 * 1024);
    __type(key, struct key_t);
    __type(value, struct value_t);
} page_access SEC(".maps");

SEC("tracepoint/exceptions/page_fault_user")
int trace_page_fault(struct trace_event_raw_x86_exceptions *ctx)
{
    // Đọc địa chỉ từ context
    __u64 vaddr = ctx->address;
    
    // Lấy PID (32-bit cao của tgid_pid)
    __u32 pid = bpf_get_current_pid_tgid() >> 32;
    __u64 ts = bpf_ktime_get_ns();

    struct key_t key = {
        .pid = pid,
        .vaddr = vaddr,
    };
    
    struct value_t val = {
        .timestamp_ns = ts,
    };

    // Ghi vào map để Userspace (Zig) quét và thực hiện Predictive Swap
    bpf_map_update_elem(&page_access, &key, &val, BPF_ANY);
    
    return 0;
}

char _license[] SEC("license") = "GPL";