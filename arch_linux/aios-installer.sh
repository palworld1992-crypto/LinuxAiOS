#!/bin/bash
set -euo pipefail

AIOS_USER="aios"
AIOS_HOME="/home/aios"
AIOS_PREEMPT="${AIOS_PREEMPT:-full}"

setup_user() {
    if id "$AIOS_USER" &>/dev/null; then
        echo "User $AIOS_USER already exists"
    else
        echo "Creating user $AIOS_USER..."
        sudo useradd -m -s /bin/bash "$AIOS_USER"
    fi
}

setup_ssh() {
    mkdir -p "$AIOS_HOME/.ssh"
    chmod 700 "$AIOS_HOME/.ssh"
    
    if [[ -f "$HOME/.ssh/id_rsa.pub ]]; then
        cp "$HOME/.ssh/id_rsa.pub" "$AIOS_HOME/.ssh/authorized_keys"
        chmod 600 "$AIOS_HOME/.ssh/authorized_keys"
        chown -R "$AIOS_USER:$AIOS_USER" "$AIOS_HOME/.ssh"
        echo "SSH key configured"
    else
        echo "WARNING: No SSH key found, skipping SSH setup"
    fi
}

setup_systemd_services() {
    local services=("aios-supervisor" "aios-main" "aios-transport" "aios-master-tunnel")
    
    for svc in "${services[@]}"; do
        cat <<EOF | sudo tee "/etc/systemd/system/${svc}.service" > /dev/null
[Unit]
Description=AIOS ${svc}
After=network.target

[Service]
Type=simple
User=$AIOS_USER
WorkingDirectory=$AIOS_HOME
ExecStart=$AIOS_HOME/target/release/${svc}
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
        sudo systemctl enable "${svc}.service" 2>/dev/null || true
    done
    echo "Systemd services created and enabled"
}

setup_zram() {
    cat <<EOF | sudo tee /etc/systemd/zram-generator.conf > /dev/null
[zram0]
zram-size = ram / 2
compression-algorithm = zstd
swap-priority = 100
EOF
    sudo systemctl daemon-reload
    sudo systemctl start systemd-zram-setup@zram0.service 2>/dev/null || true
    echo "Zram configured"
}

setup_grub_preempt() {
    if [[ "$AIOS_PREEMPT" == "full" ]]; then
        if grep -q "preempt=full" /proc/cmdline 2>/dev/null; then
            echo "preempt=full already in kernel cmdline"
            return
        fi
        
        local grub_file="/etc/default/grub"
        if [[ -f "$grub_file" ]]; then
            if grep -q 'GRUB_CMDLINE_LINUX_DEFAULT=' "$grub_file"; then
                sudo sed -i 's/GRUB_CMDLINE_LINUX_DEFAULT="\([^"]*\)"/GRUB_CMDLINE_LINUX_DEFAULT="\1 preempt=full"/' "$grub_file"
                if command -v update-grub &>/dev/null; then
                    sudo update-grub
                elif command -v grub-mkconfig &>/dev/null; then
                    sudo grub-mkconfig -o /boot/grub/grub.cfg
                fi
                echo "GRUB updated with preempt=full"
            fi
        else
            echo "WARNING: GRUB config not found, skipping preempt setup"
        fi
    else
        echo "Skipping preempt setup (AIOS_PREEMPT=$AIOS_PREEMPT)"
    fi
}

create_directories() {
    sudo mkdir -p /var/lib/aios/{snapshots,data,logs}
    sudo chown -R "$AIOS_USER:$AIOS_USER" /var/lib/aios
    echo "Directories created"
}

main() {
    echo "Running AIOS installer..."
    setup_user
    setup_ssh
    create_directories
    setup_systemd_services
    setup_zram
    setup_grub_preempt
    
    echo "Installation complete!"
    echo "Reboot to apply changes, then run: sudo systemctl start aios-supervisor"
}

main "$@"