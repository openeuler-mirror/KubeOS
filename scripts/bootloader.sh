#!/bin/bash
set -eu
set -o pipefail
set -x
ARCH=`arch`

function install_grub2_x86 ()
{
    # make efi file, and save in FAT16 partition, to support UEFI boot mode
    cp -r /usr/lib/grub/x86_64-efi boot/efi/EFI/openEuler
    eval "grub2-mkimage -d /usr/lib/grub/x86_64-efi -O x86_64-efi --output=/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"

    mkdir -p /boot/EFI/BOOT/
    cp -f /boot/efi/EFI/openEuler/grubx64.efi /boot/efi/EFI/BOOT/BOOTX64.EFI
}

function install_grub2_efi ()
{
    cp -r /usr/lib/grub/arm64-efi /boot/efi/EFI/openEuler/
    eval "grub2-mkimage -d /usr/lib/grub/arm64-efi -O arm64-efi --output=/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"

    mkdir -p /boot/EFI/BOOT/
    cp -f /boot/efi/EFI/openEuler/grubaa64.efi /boot/efi/EFI/BOOT/BOOTAA64.EFI
}

if [ $ARCH == "x86_64" ]; then
    install_grub2_x86
fi

if [ $ARCH == "aarch64" ]; then
    install_grub2_efi
fi
