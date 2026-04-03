#!/bin/bash
set -euo pipefail

check_kernel_config() {
    local config_file="$1"
    local missing=()
    
    local required_flags=(
        "CONFIG_BPF"
        "CONFIG_BPF_SYSCALL"
        "CONFIG_CGROUP_BPF"
        "CONFIG_BPF_EVENTS"
        "CONFIG_DEBUG_INFO_BTF"
        "CONFIG_BPF_JIT"
        "CONFIG_BPF_JIT_ALWAYS_ON"
    )
    
    for flag in "${required_flags[@]}"; do
        if grep -q "^${flag}=y" "$config_file" 2>/dev/null; then
            echo "  [OK] $flag"
        else
            missing+=("$flag")
            echo "  [MISSING] $flag"
        fi
    done
    
    if [[ ${#missing[@]} -eq 0 ]]; then
        echo ""
        echo "All required kernel features are enabled!"
        return 0
    else
        echo ""
        echo "WARNING: ${#missing[@]} required features are missing"
        return 1
    fi
}

find_config() {
    if [[ -f /proc/config.gz ]]; then
        echo "/proc/config.gz"
    elif [[ -f /boot/config-"$(uname -r)" ]]; then
        echo "/boot/config-$(uname -r)"
    elif [[ -f /lib/modules/"$(uname -r)"/build/.config ]]; then
        echo "/lib/modules/$(uname -r)/build/.config"
    else
        echo "ERROR: Cannot find kernel config file"
        echo "Try: modprobe configs"
        return 1
    fi
}

main() {
    echo "Checking kernel features for eBPF..."
    echo ""
    
    local config
    config=$(find_config) || exit 1
    
    echo "Using config: $config"
    echo "Required flags:"
    echo ""
    
    if [[ "$config" == *.gz ]]; then
        zgrep -E "^(CONFIG_BPF|CONFIG_CGROUP_BPF|CONFIG_BPF_JIT)" "$config" | check_kernel_config "$config"
    else
        check_kernel_config "$config"
    fi
    
    local status=$?
    
    if [[ $status -ne 0 ]]; then
        echo ""
        echo "Recommendation: Install linux-zen or linux-hardened kernel"
        echo "  pacman -S linux-zen"
        echo "OR"
        echo "  pacman -S linux-hardened"
    fi
    
    exit $status
}

main "$@"