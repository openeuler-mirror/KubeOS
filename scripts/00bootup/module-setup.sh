#!/bin/bash

check() {
    return 0
}

depends() {
    echo systemd
}

install() {
    inst_multiple -o grub2-mkimage mkfs.ext4 mkfs.vfat lsblk tar cpio gunzip lspci parted dhclient ifconfig curl hwinfo head tee arch df awk route 
    inst_hook mount 00 "$moddir/mount.sh"
    inst_simple "$moddir/mount.sh" "/mount.sh"
    inst_simple "$moddir/Global.cfg" "/Global.cfg"
}

installkernel() {
    hostonly='' \
        instmods \
        =drivers/ata \
        =drivers/nvme \
        =drivers/scsi \
        =drivers/net \
        =fs/fat \
        =fs/nls
}
  
