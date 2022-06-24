#!/bin/bash
set -eu
set -o pipefail
set -x
ARCH=`arch`

function install_grub2_x86 ()
{
    # make boot.img/core.img and setup, to support legacy boot mode
    GRUBNAME=$(which grub2-install)
    echo "Installing GRUB2..."
    GRUB_OPTS=${GRUB_OPTS:-"--force"}
    GRUB_OPTS="$GRUB_OPTS --target=i386-pc"

    $GRUBNAME --modules="biosdisk part_msdos" $GRUB_OPTS $DEVICE

    # make efi file, and save in FAT16 partition, to support UEFI boot mode
    cp -r /usr/lib/grub/x86_64-efi boot/efi/EFI/openEuler
    eval "grub2-mkimage -d /usr/lib/grub/x86_64-efi -O x86_64-efi --output=/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,msdos1)/EFI/openEuler' fat part_gpt part_msdos linux"

    mkdir -p /boot/EFI/BOOT/
    cp -f /boot/efi/EFI/openEuler/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
}

function install_grub2_efi ()
{
    cp -r /usr/lib/grub/arm64-efi /boot/efi/EFI/openEuler/
    eval "grub2-mkimage -d /usr/lib/grub/arm64-efi -O arm64-efi --output=/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,msdos1)/EFI/openEuler' fat part_gpt part_msdos linux"

    mkdir -p /boot/EFI/BOOT/
    cp -f /boot/efi/EFI/openEuler/grubaa64.efi /boot/efi/EFI/BOOT/BOOTAA64.EFI
}

if [ $ARCH == "x86_64" ]; then
    install_grub2_x86
fi

if [ $ARCH == "aarch64" ]; then
    install_grub2_efi
fi
