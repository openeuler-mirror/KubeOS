#!/bin/bash

# 日志文件
LOGFILE="/var/log/configure-aide-audit.log"
mkdir -p "$(dirname "$LOGFILE")"
> "$LOGFILE"  # 清空旧日志

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" | tee -a "$LOGFILE"
}

# 配置函数：检查并配置一行
configure_line() {
    local file="$1"
    local line="$2"

    # 提取参数名（第一列），用于匹配
    local param_name=$(echo "$line" | awk '{print $1}')
    param_name="${param_name%%=*}"

    # 确保目录存在
    mkdir -p "$(dirname "$file")"

    # 如果文件不存在，创建它
    if [ ! -f "$file" ]; then
        touch "$file"
        log "创建 - 文件 $file"
    fi

    # 1. 查找所有包含该参数名的行（包括被注释的和未注释的）
    # 使用 grep -n 获取行号和行内容
    local matches=$(grep -n "^$param_name" "$file" 2>/dev/null)

    if [ -n "$matches" ]; then
        # 获取第一个匹配到的行号和内容
        local first_match_line=$(echo "$matches" | head -n1)
        local matched_line_number=$(echo "$first_match_line" | cut -d: -f1)
        local matched_content=$(echo "$first_match_line" | cut -d: -f2-)

        # --- 核心逻辑：标准化对比 ---
        # 1. 预期行标准化：去除首尾空格，去除开头的 # 和空格
        local expected_clean=$(echo "$line" | sed 's/^[[:space:]]*//; s/#.*$//' | sed 's/[[:space:]]*$//')

        # 2. 匹配行标准化：同样去除开头的 # 和空格，去除首尾空格
        local matched_clean=$(echo "$matched_content" | sed 's/^[[:space:]]*//; s/#.*$//' | sed 's/[[:space:]]*$//')

        if [ "$expected_clean" = "$matched_clean" ]; then
            # 情况 A：内容一致（无论是否被注释），跳过
            log "跳过 - $file 已配置: $line"
            return
        else
            # 情况 B：内容不一致（值不同，或注释状态不同导致逻辑差异），执行替换
            # 逻辑：删除原行，在行号位置插入新行
            awk -v new_line="$line" -v line_number="$matched_line_number" '
                NR == line_number {
                    print new_line
                    next
                }
                { print }
            ' "$file" > "${file}.tmp"

            mv "${file}.tmp" "$file"
            log "$file 配置值已修正: $line"
            return
        fi
    fi

    # 3. 如果文件里完全没找到该参数名，追加到文件末尾
    echo "$line" >> "$file"
    log "新增 - $file 配置: $line"
}

# 配置函数：检查并配置一行
configure_line_multivalue() {
    local file="$1"
    local line="$2"

    if [ -f "$file" ]; then
        # 如果配置已经存在且与预期一致 → 跳过
        if grep -Fq "$line" "$file"; then
            log "跳过 - $file 已配置正确: $line"
            return
        fi
    fi

    # 如果文件不存在，或配置未存在 → 新增
    mkdir -p "$(dirname "$file")"
    if [ ! -f "$file" ]; then
        touch "$file"
        log "创建 - 文件 $file"
    fi

    echo "$line" >> "$file"
    log "$file 新增 - 配置: $line"
}

# --- 新增函数：处理 SSSD 配置文件 ---
configure_sssd() {
    local config_file="/etc/sssd/sssd.conf"
    local nss_timeout="86400"
    local pam_expiration="1"

    # 如果配置文件不存在，则创建并写入预期配置
    if [[ ! -f "$config_file" ]]; then
        cat > "$config_file" <<EOF
[nss]
memcache_timeout=$nss_timeout

[pam]
offline_credentials_expiration=$pam_expiration
EOF
        log "已创建 $config_file 并写入默认配置"
        return 0
    fi

    # 函数：检测并修改 [nss] 下的 memcache_timeout
    configure_nss() {
        if ! grep -qE '^\[nss\]' "$config_file"; then
            # [nss] 不存在，创建并插入
            if grep -qE '^\[pam\]' "$config_file"; then
                # 如果 [pam] 存在，将 [nss] 插入到 [pam] 之前
                sed -i "/^\[pam\]/i [nss]
memcache_timeout=$nss_timeout
" "$config_file"
            else
                # 如果 [pam] 也不存在，直接追加
                cat >> "$config_file" <<EOF

[nss]
memcache_timeout=$nss_timeout
EOF
            fi
            log "$config_file:已创建 [nss] 段并设置 memcache_timeout=$nss_timeout"
        else
            # [nss] 存在，检查 memcache_timeout
            if ! grep -qE '^memcache_timeout=' "$config_file"; then
                # 值不存在，插入到 [nss] 下一行
                sed -i "/^\[nss\]/a memcache_timeout=$nss_timeout" "$config_file"
                log "$config_file:已在 [nss] 下新增 memcache_timeout=$nss_timeout"
            else
                # 检查值是否正确
                local current_val=$(grep -E '^memcache_timeout=' "$config_file" | awk -F= '{print $2}')
                if [[ "$current_val" != "$nss_timeout" ]]; then
                    sed -i "s/^memcache_timeout=.*/memcache_timeout=$nss_timeout/" "$config_file"
                    log "$config_file:已修正 [nss] 下的 memcache_timeout 为 $nss_timeout"
                # 如果配置已经存在且与预期一致 → 跳过
                else
                    log "$config_file:跳过 - [nss] 下的 memcache_timeout已配置正确: $nss_timeout"
                fi
            fi
        fi
    }

    # 函数：检测并修改 [pam] 下的 offline_credentials_expiration
    configure_pam() {
        if ! grep -qE '^\[pam\]' "$config_file"; then
            # [pam] 不存在，创建并插入
            if grep -qE '^\[nss\]' "$config_file"; then
                # 如果 [nss] 存在，在其下一行插入 [pam]
                sed -i "/^\[nss\]/a [pam]
offline_credentials_expiration=$pam_expiration
" "$config_file"
            else
                # 如果 [nss] 也不存在，直接追加
                cat >> "$config_file" <<EOF

[pam]
offline_credentials_expiration=$pam_expiration
EOF
            fi
            log "$config_file:已创建 [pam] 段并设置 offline_credentials_expiration=$pam_expiration"
        else
            # [pam] 存在，检查 offline_credentials_expiration
            if ! grep -qE '^offline_credentials_expiration=' "$config_file"; then
                # 值不存在，插入到 [pam] 下一行
                sed -i "/^\[pam\]/a offline_credentials_expiration=$pam_expiration" "$config_file"
                log "$config_file:已在 [pam] 下新增 offline_credentials_expiration=$pam_expiration"
            else
                # 检查值是否正确
                local current_val=$(grep -E '^offline_credentials_expiration=' "$config_file" | awk -F= '{print $2}')
                if [[ "$current_val" != "$pam_expiration" ]]; then
                    sed -i "s/^offline_credentials_expiration=.*/offline_credentials_expiration=$pam_expiration/" "$config_file"
                    log "$config_file:已修正 [pam] 下的 offline_credentials_expiration 为 $pam_expiration"
                # 如果配置已经存在且与预期一致 → 跳过
                else
                    log "$config_file:跳过 - [pam] 下的 offline_credentials_expiration已配置正确: $pam_expiration"
                fi
            fi
        fi
    }

    # 调用函数
    configure_nss
    configure_pam

    # 设置文件权限（安全最佳实践）
    if [[ -f "$config_file" ]]; then
        chmod 600 "$config_file"
        chown root:root "$config_file"
    fi
}

# --- 核心逻辑：PAM 安全加固 (删除 nullok) ---
pam_security_conf() {
    log "开始扫描 /etc/pam.d/ 目录，检查空密码配置 (nullok)..."

    local files_scanned=0

    for pam_file in /etc/pam.d/*; do
        [ -f "$pam_file" ] || continue

        # 检查是否包含 pam_unix.so 且包含 nullok
        if grep -q "pam_unix.so" "$pam_file" && grep -q "nullok" "$pam_file"; then
            files_scanned=$((files_scanned + 1))

            # 执行删除：移除行中的 "nullok" 单词
            # \bnullok\b 确保只匹配单词 nullok，不匹配 nonullok 等
            log "发现空密码配置风险：$pam_file (含 nullok)"

            # sed -i 直接修改文件
            # 逻辑：将 "nullok" 替换为空，并清理可能产生的多余空格
            sed -i -E 's/\bnullok\b[[:space:]]*//g; s/[[:space:]]{2,}/ /g' "$pam_file"

            log "已移除 nullok 配置：$pam_file"
        fi
    done

    if [ $files_scanned -eq 0 ]; then
        log "扫描完成：未发现 /etc/pam.d/ 中存在 nullok 配置，系统已安全。"
    else
        log "扫描完成：共发现 $files_scanned 个文件存在空密码风险，并已修复。"
    fi
}

# 配置函数：检查并配置一行
configure_audit() {
    local file="$1"
    local line="$2"

    if [ -f "$file" ]; then
        # 1. 检查配置是否存在（注意：加上 -- 防止 line 以 - 开头被误读为参数）
        if grep -Fq -- "$line" "$file"; then
            log "跳过 - $file 已配置: $line"
            return
        fi
    fi

    # 3. 如果文件不存在，或配置未存在 → 新增
    mkdir -p "$(dirname "$file")"
    if [ ! -f "$file" ]; then
        touch "$file"
        log "创建 - 文件 $file"
    fi

    # 确保不重复追加（虽然上面检查过，但双重保险）
    if ! grep -Fq -- "$line" "$file"; then
        echo "$line" >> "$file"
        log "$file 新增 - 配置: $line"
    else
        log "跳过 - $file 已存在配置: $line"
    fi
}

# 执行配置
log " 开始配置..."

# 1.执行 SSSD 配置
log "--- 1.正在处理 SSSD 配置 ---"
configure_sssd
log "--- SSSD 已按要求配置 ---"

# 2. 处理 PAM 安全加固
log "--- 2.正在执行 PAM 安全加固 ---"
pam_security_conf
log "--- PAM已按要求配置 ---"

# 3. 处理 常规配置项
log "--- 3.正在执行 常规配置项 ---"

configure_line "/etc/sysctl.conf" "kernel.dmesg_restrict = 1"
configure_line "/etc/yum.conf" "clean_requirements_on_remove=True"
configure_line "/etc/sysctl.d/99-stig.conf" "kernel.randomize_va_space=2"
configure_line "/etc/sysctl.d/99-stig.conf" "kernel.kptr_restrict=1"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv4.conf.all.accept_redirects=0"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv4.conf.default.accept_redirects=0"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv4.conf.all.send_redirects=0"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv4.conf.default.send_redirects=0"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv6.conf.all.accept_redirects=0"
configure_line "/etc/sysctl.d/99-stig.conf" "net.ipv6.conf.default.accept_redirects=0"
configure_line "/etc/ssh/sshd_config" "Banner /etc/issue/"
configure_line "/etc/ssh/sshd_config" "ClientAliveCountMax 1"
configure_line "/etc/ssh/sshd_config" "ClientAliveInterval 600"
configure_line "/etc/ssh/sshd_config" "X11Forwarding no"
configure_line "/etc/ssh/sshd_config" "PermitRootLogin no"
configure_line "/etc/ssh/sshd_config" "LogLevel VERBOSE"
configure_line "/etc/ssh/sshd_config" "PrintLastLog yes"
configure_line "/etc/ssh/sshd_config" "IgnoreUserKnownHosts yes"
configure_line "/etc/ssh/sshd_config" "StrictModes yes"
configure_line "/etc/ssh/sshd_config" "PermitEmptyPasswords no"
configure_line "/etc/ssh/sshd_config" "PermitUserEnvironment no"
configure_line "/etc/ssh/sshd_config" "Ciphers aes256-ctr,aes192-ctr,aes128-ctr"
configure_line "/etc/ssh/sshd_config" "MACs hmac-sha2-512,hmac-sha2-256"
configure_line "/etc/ssh/sshd_config" "KexAlgorithms ecdh-sha2-nistp256,ecdh-sha2-nistp384,ecdh-sha2-nistp521,diffie-hellman-group-exchange-sha256"
configure_line "/etc/login.defs" "FAIL_DELAY 5"
configure_line "/etc/login.defs" "PASS_MIN_DAYS 1"
configure_line "/etc/login.defs" "PASS_MAX_DAYS   7"
configure_line "/etc/login.defs" "SHA_CRYPT_MIN_ROUNDS 5000"
configure_line "/etc/login.defs" "SHA_CRYPT_MAX_ROUNDS 5000"
configure_line "/etc/login.defs" "CREATE_HOME yes"
configure_line "/etc/login.defs" "UMASK 077"
configure_line_multivalue "/etc/sudoers" "Defaults !targetpw"
configure_line_multivalue "/etc/sudoers" "Defaults !rootpw"
configure_line_multivalue "/etc/sudoers" "Defaults !runaspw"
configure_line_multivalue "/etc/sudoers" "Defaults timestamp_timeout=0"
configure_line "/etc/default/useradd" "INACTIVE=35"
configure_line_multivalue "/etc/pam.d/login" "session required pam_lastlog.so showfailed"
configure_line "/etc/profile.d/autologout.sh" "TMOUT=900"
configure_line "/etc/profile.d/autologout.sh" "readonly TMOUT"
configure_line "/etc/profile.d/autologout.sh" "export TMOUT"
configure_line_multivalue "/etc/pam.d/common-account" "account required pam_faillock.so"
configure_line_multivalue "/etc/pam.d/common-auth" "auth required pam_faildelay.so delay=5000000"
configure_line_multivalue "/etc/pam.d/common-auth" "auth required pam_faillock.so onerr=fail silent audit deny=3"
configure_line_multivalue "/etc/pam.d/common-auth" "auth sufficient pam_pkcs11.so"
configure_line "/etc/selinux/config" "SELINUXTYPE=targeted"
configure_line "/etc/sudoers" "#includedir /etc/sudoers.d"
configure_line "/etc/pam.d/common-password" "password requisite pam_pwquality.so"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so ucredit=-1"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so lcredit=-1"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so dcredit=-1"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so ocredit=-1"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so minlen=15"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwquality.so difok=8"
configure_line_multivalue "/etc/pam.d/common-password" "password requisite pam_pwhistory.so remember=5 use_authtok"
configure_line_multivalue "/etc/pam.d/common-password" "password required pam_unix.so sha512"
configure_line "/etc/pam_pkcs11/pam_pkcs11.conf" "cert_policy = ca,ocsp_on,signature,crl_auto;"
configure_line_multivalue "/etc/aide.conf" "# audit tools"
configure_line "/etc/aide.conf" "/usr/sbin/auditctl p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/auditd p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/ausearch p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/aureport p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/autrace p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/audispd p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/aide.conf" "/usr/sbin/augenrules p+i+n+u+g+s+b+acl+selinux+xattrs+sha512"
configure_line "/etc/cron.weekly/aide" "0 0 * * * /usr/sbin/aide --check | /bin/mail -s \"\$HOSTNAME - Weekly AIDE integrity check run\" root@example_server_name.mil"
configure_line "/etc/cron.daily/aide" "0 0 * * * /usr/sbin/aide --check | /bin/mail -s \"\$HOSTNAME - Daily AIDE integrity check run\" root@example_server_name.mil"
configure_line "/etc/audit/plugins.d/au-remote.conf" "active = yes"
configure_line "/etc/audit/auditd.conf" "space_left = 25%"
configure_line "/etc/audit/auditd.conf" "disk_full_action = HALT"
configure_line "/etc/audit/audisp-remote.conf" "network_failure_action = syslog"
configure_line "/etc/audit/audisp-remote.conf" "disk_full_action = syslog"
configure_line "/etc/audit/audisp-remote.conf" "remote_server = <ip_address>"
configure_line "/etc/audit/audisp-remote.conf" "enable_krb5 = yes"
configure_line "/etc/security/limits.conf" "* hard maxlogins 10"
configure_line "/etc/audit/rules.d/audit.rules" "/etc/sudoers"
configure_line "/etc/audit/rules.d/audit.rules" "/etc/sudoers.d"
log "--- 常规配置项 已按要求配置 ---"

# 4.执行 AUDIT 配置
log "--- 4.正在处理 AUDIT 配置 ---"
configure_audit "/etc/audit/rules.d/audit.rules" "-D"
configure_audit "/etc/audit/rules.d/audit.rules" "-b 8192"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chacl -F perm=x -F auid>=1000 -F auid!=unset -k prim_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chage -F perm=x -F auid>=1000 -F auid!=unset -k privileged-chage"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chcon -F perm=x -F auid>=1000 -F auid!=unset -k prim_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chfn -F perm=x -F auid>=1000 -F auid!=unset -k privileged-chfn"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chmod -F perm=x -F auid>=1000 -F auid!=unset -k prim_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/chsh -F perm=x -F auid>=1000 -F auid!=unset -k privileged-chsh"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/crontab -F perm=x -F auid>=1000 -F auid!=unset -k privileged-crontab"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/gpasswd -F perm=x -F auid>=1000 -F auid!=unset -k privileged-gpasswd"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /sbin/insmod -p x -k modules "
configure_audit "/etc/audit/rules.d/audit.rules" "-w /usr/bin/kmod -p x -k modules"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /sbin/modprobe -p x -k modules "
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/newgrp -F perm=x -F auid>=1000 -F auid!=unset -k privileged-newgrp"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/sbin/pam_timestamp_check -F perm=x -F auid>=1000 -F auid!=unset -k privileged-pam_timestamp_check"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/passwd -F perm=x -F auid>=1000 -F auid!=unset -k privileged-passwd"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/rm -F perm=x -F auid>=1000 -F auid!=unset -k prim_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /sbin/rmmod -p x -k modules"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/setfacl -F perm=x -F auid>=1000 -F auid!=unset -k prim_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/ssh-agent -F perm=x -F auid>=1000 -F auid!=unset -k privileged-ssh-agent"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/lib/ssh/ssh-keysign -F perm=x -F auid>=1000 -F auid!=unset -k privileged-ssh-keysign"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/su -F perm=x -F auid>=1000 -F auid!=unset -k privileged-priv_change"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/sudo -F perm=x -F auid>=1000 -F auid!=unset -k privileged-sudo"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/bin/sudoedit -F perm=x -F auid>=1000 -F auid!=-1 -F key=privileged-sudoedit"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -S all -F path=/sbin/unix_chkpwd -F perm=x -F auid>=1000 -F auid!=-1 -F key=privileged-unix-chkpwd"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -S all -F path=/sbin/unix2_chkpwd -F perm=x -F auid>=1000 -F auid!=-1 -F key=privileged-unix2-chkpwd"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/sbin/usermod -F perm=x -F auid>=1000 -F auid!=unset -k privileged-usermod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/group -p wa -k account_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/security/opasswd -p wa -k account_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/passwd -p wa -k account_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/shadow -p wa -k account_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S chmod,fchmod,fchmodat -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S chmod,fchmod,fchmodat -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S chown,fchown,fchownat,lchown -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S chown,fchown,fchownat,lchown -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S creat,open,openat,open_by_handle_at,truncate,ftruncate -F exit=-EPERM -F auid>=1000 -F auid!=unset -k perm_access"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S creat,open,openat,open_by_handle_at,truncate,ftruncate -F exit=-EPERM -F auid>=1000 -F auid!=unset -k perm_access"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S creat,open,openat,open_by_handle_at,truncate,ftruncate -F exit=-EACCES -F auid>=1000 -F auid!=unset -k perm_access"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S creat,open,openat,open_by_handle_at,truncate,ftruncate -F exit=-EACCES -F auid>=1000 -F auid!=unset -k perm_access"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S delete_module -F auid>=1000 -F auid!=unset -k unload_module"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S delete_module -F auid>=1000 -F auid!=unset -k unload_module"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S init_module,finit_module -F auid>=1000 -F auid!=unset -k moduleload"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S init_module,finit_module -F auid>=1000 -F auid!=unset -k moduleload"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S mount -F auid>=1000 -F auid!=unset -k privileged-mount"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S mount -F auid>=1000 -F auid!=unset -k privileged-mount"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 --a always,exit -F arch=b32 -S setxattr,fsetxattr,lsetxattr,removexattr,fremovexattr,lremovexattr -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S setxattr,fsetxattr,lsetxattr,removexattr,fremovexattr,lremovexattr -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S umount -F auid>=1000 -F auid!=unset -k privileged-umount"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S umount2 -F auid>=1000 -F auid!=unset -k privileged-umount"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S umount2 -F auid>=1000 -F auid!=unset -k privileged-umount"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S unlink,unlinkat,rename,renameat,rmdir -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S unlink,unlinkat,rename,renameat,rmdir -F auid>=1000 -F auid!=unset -k perm_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S execve -C uid!=euid -F euid=0 -k setuid"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S execve -C uid!=euid -F euid=0 -k setuid"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b32 -S execve -C gid!=egid -F egid=0 -k setgid"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F arch=b64 -S execve -C gid!=egid -F egid=0 -k setgid"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /var/log/lastlog -p wa -k logins"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /var/log/tallylog -p wa -k logins"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/sudoers -p wa -k privileged-actions"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /etc/sudoers.d -p wa -k privileged-actions"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/sbin/setfiles -F perm=x -F auid>=1000 -F auid!=unset -k privileged-unix-update"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/sbin/semanage -F perm=x -F auid>=1000 -F auid!=unset -k privileged-unix-update"
configure_audit "/etc/audit/rules.d/audit.rules" "-a always,exit -F path=/usr/sbin/setsebool -F perm=x -F auid>=1000 -F auid!=unset -k privileged-unix-update"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /run/utmp -p wa -k login_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /var/log/btmp -p wa -k login_mod"
configure_audit "/etc/audit/rules.d/audit.rules" "-w /var/log/wtmp -p wa -k login_mod"
log "--- AUDIT 已按要求配置 ---"


#审计日志/配置文件权限设置正确(可以通过chmod/chown设置文件权限)
mkdir -p /var/log/audit && [ ! -f /var/log/audit/audit.log ] && touch /var/log/audit/audit.log; chown root:root /var/log/audit && chmod 600 /var/log/audit; chown root:root /var/log/audit/audit.log && chmod 600 /var/log/audit/audit.log
mkdir -p /etc/audit && [ ! -f /etc/audit/audit.rules ] && touch /etc/audit/audit.rules; chown root:root /etc/audit/audit.rules && chmod 640 /etc/audit/audit.rules
mkdir -p /etc/audit/rules.d && [ ! -f /etc/audit/rules.d/audit.rules ] && touch /etc/audit/rules.d/audit.rules; chown root:root /etc/audit/rules.d/audit.rules && chmod 640 /etc/audit/rules.d/audit.rules

#审计相关命令权限设置正确(可以通过chmod/chown设置文件权限)
chmod 750 /usr/sbin/auditctl
chmod 750 /usr/sbin/auditd
chmod 755 /usr/sbin/ausearch
chmod 755 /usr/sbin/aureport
chmod 750 /usr/sbin/autrace
chmod 750 /usr/sbin/augenrules

chown root:root /usr/sbin/auditctl
chown root:root /usr/sbin/auditd
chown root:root /usr/sbin/ausearch
chown root:root /usr/sbin/aureport
chown root:root /usr/sbin/autrace
chown root:root /usr/sbin/augenrules

#The file permissions under /var/log/ should be root:root:640
find /var/log -perm /137 ! -name '*[bw]tmp' ! -name '*lastlog' -type f -exec chmod 640 '{}' \;

#禁止 ctrl-alt-del.target服务
systemctl disable ctrl-alt-del.target
systemctl mask ctrl-alt-del.target
systemctl daemon-reload

#打开系统FIPS模式
sha512hmac /boot/vmlinuz > /boot/.vmlinuz.hmac
fips-mode-setup --enable

#默认开启enforcing模式，在/etc/selinux/config中配置"SELINUX=enforcing"不起作用
setfiles -c /etc/selinux/targeted/policy/policy.33 /etc/selinux/targeted/contexts/files/file_contexts  /
find / -type f -name grub.cfg 2>/dev/null | while read file; do
  if grep -q 'enforcing=0' "$file"; then
    continue
  elif grep -q 'selinux=0' "$file"; then
    sed -i 's/selinux=0/enforcing=0/g' "$file"
  else
    sed -i '/vmlinuz/ { s/$/ enforcing=0/ }' "$file"
  fi
done

echo "所有任务执行完毕！"
