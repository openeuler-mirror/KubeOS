#!/bin/bash
arch=$(arch)
min_size=8
log=/install.log

source /Global.cfg

function CheckSpace() {
    local disk_ava="$(parted -l | grep ${disk} | awk '{print $3}')"
    if echo "${disk_ava}" | grep [GT]B$; then
        if echo "${disk_ava}" | grep GB$; then
            disk_ava="$(echo ${disk_ava} | awk -F G '{print $1}' | awk -F . '{print $1}')"
            if [ "${disk_ava}" -lt ${min_size} ]; then
                echo "The available disk space is not enough, at least ${min_size}GB." | tee -a ${log}
                return 1
            fi
        fi
    else
        echo "The available disk space is not enough, at least ${min_size}G." | tee -a ${log}
        return 1
    fi

    return 0
}

function GetDisk() {
    disks=$(hwinfo --disk --short | grep -vi "^disk" | awk '{print $1}')
    if [ ! -z ${disks} ]; then
        if [ ! -z ${disk} ] && echo "${disks[@]}" | grep -wq "${disk}" ; then
            echo "${disk} exists, start partition"  | tee -a ${log}
        else
            echo "disk not exist, choose default disk"  | tee -a ${log}
            disk=$(echo ${disks[0]})
        fi
    else
        echo "no disk found" | tee -a ${log}
        return 1
    fi
    

    CheckSpace
    if [ $? -ne 0 ]; then
        echo "no enough space on ${disk}" | tee -a ${log}
        return 1
    fi

    return 0
}

function PartitionAndFormatting() {
    echo "Partitioning and formatting disk $disk..."
    # partition and format
    parted ${disk} -s mklabel gpt >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    parted ${disk} -s mkpart primary fat16 1M 100M >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    parted ${disk} -s mkpart primary ext4 100M 2600M >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    parted ${disk} -s mkpart primary ext4 2600M 5100M >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    parted ${disk} -s mkpart primary ext4 5100M 100% >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    parted ${disk} -s set 1 boot on >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "partition failed" | tee -a ${log}
        return 1
    fi

    mkfs.vfat -n "BOOT" ${disk}1 >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "format failed" | tee -a ${log}
        return 1
    fi

    mkfs.ext4 -L "ROOT-A" ${disk}2 >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "format failed" | tee -a ${log}
        return 1
    fi

    mkfs.ext4 -L "ROOT-B" ${disk}3 >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "format failed" | tee -a ${log}
        return 1
    fi

    mkfs.ext4 -L "PERSIST" ${disk}4 >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "format failed" | tee -a ${log}
        return 1
    fi

    return 0
}

function InitNetwork() {
    echo "Initializing network..."
    # 获取网卡信息，默认只有一个网卡
    net_name=`ifconfig -a | awk '{print $1}' | grep : | grep '^e' | awk -F: '{print $1}'`
    # dhclient --timeout 60 >> ${log} 2>&1

    ifconfig ${net_name} up
    if [ $? -ne 0 ]; then
        echo "load net card failed" | tee -a ${log}
        return 1
    fi
    sleep 3

    ifconfig ${net_name} ${local_ip} netmask 255.255.255.0  >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "ip set failed" | tee -a ${log}
        return 1
    fi
    sleep 3

    route add default gw ${route_ip} >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "add route failed" | tee -a ${log}
        return 1
    fi
    sleep 3

    

    return 0
}

function MountRoot() {
    echo "Mounting rootfs..."
    # mount rootfs
    mount ${disk}2 /sysroot >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "mount rootfs failed" | tee -a ${log}
        return 1
    fi

    return 0
}

function MountPersist() {
    echo "Mounting persist"
    mkdir /persist
    mount ${disk}4 /persist >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "mount persist failed" | tee -a ${log}
        return 1
    fi
    mkdir /persist/{var,etc,etcwork}
    mkdir -p /persist/etc/KubeOS/certs
    return 0
}

function MountBoot() {
    echo "Mounting boot"
    mkdir -p /sysroot/boot/efi
    mount ${disk}1 /sysroot/boot/efi >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "mount boot failed" | tee -a ${log}
        return 1
    fi
    return 0
}

function GetRootfs() {
    echo "Downloading rootfs..."

    curl -o /persist/${rootfs_name} http://${server_ip}/${rootfs_name}
    if [ ! -e "/persist/${rootfs_name}" ]; then
        echo "download rootfs failed" | tee -a ${log}
        return 1
    fi

    tar -xvf /persist/${rootfs_name} -C /sysroot
    if [ $? -ne 0 ]; then
        echo "decompose rootfs failed" | tee -a ${log}
        return 1
    fi

    rm -rf /persist/${rootfs_name}

    return 0
}

function Inst_Grub2_x86() {
    # copy the files that boot need
    cp -r /sysroot/usr/lib/grub/x86_64-efi /sysroot/boot/efi/EFI/openEuler
    eval "grub2-mkimage -d /sysroot/usr/lib/grub/x86_64-efi -O x86_64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubx64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "grub2-mkimage on x86 failed" | tee -a ${log}
        return 1
    fi
    
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubx64.efi /sysroot/boot/efi/EFI/BOOT/BOOTX64.EFI

    return 0
}

function Inst_Grub2_aarch64() {
    cp -r /sysroot/usr/lib/grub/arm64-efi /sysroot/boot/efi/EFI/openEuler/
    eval "grub2-mkimage -d /sysroot/usr/lib/grub/arm64-efi -O arm64-efi --output=/sysroot/boot/efi/EFI/openEuler/grubaa64.efi '--prefix=(,gpt1)/EFI/openEuler' fat part_gpt part_msdos linux"  >> ${log} 2>&1
    if [ $? -ne 0 ]; then
        echo "grub2-mkimage on aarch64 failed" | tee -a ${log}
        return 1
    fi
    
    mkdir -p /sysroot/boot/efi/EFI/BOOT/
    cp -f /sysroot/boot/efi/EFI/openEuler/grubaa64.efi /sysroot/boot/efi/EFI/BOOT/BOOTAA64.EFI

    return 0
}

function SetBoot() {
    # mount boot
    echo "Setting boot"

    if [ $arch == "x86_64" ];   then
        Inst_Grub2_x86
        if [ $? -ne 0 ]; then
            echo "install grub on x86 failed" | tee -a ${log}
            return 1
        fi
    fi

    if [ $arch == "aarch64" ]; then
        Inst_Grub2_aarch64
        if [ $? -ne 0 ]; then
            echo "install grub on aarch64 failed" | tee -a ${log}
            return 1
        fi
    fi

    return 0
}

function Bootup_Main() {
    # get disk
    echo "Checking disk info..." | tee -a ${log}
    GetDisk
    if [ $? -ne 0 ]; then
        echo "Checking disk info failed" | tee -a ${log}
        return 1
    fi

    # partition and format disk
    echo "Partion and formatting..." | tee -a ${log}
    PartitionAndFormatting
    if [ $? -ne 0 ]; then
        echo "Partition and formatting disk failed" | tee -a ${log}
        return 1
    fi

    # init network
    echo "Initializing network..." | tee -a ${log}
    InitNetwork
    if [ $? -ne 0 ]; then
        echo "Initializing network failed" | tee -a ${log}
        return 1
    fi

    # mount partitions
    echo "Mounting root..." | tee -a ${log}
    MountRoot
    if [ $? -ne 0 ]; then
        echo "Mounting root failed" | tee -a ${log}
        return 1
    fi

    # mount persist
    echo "Mounting persisst..." | tee -a ${log}
    MountPersist
    if [ $? -ne 0 ]; then
        echo "Mounting persist failed" | tee -a ${log}
        return 1
    fi

    # mount boot
    echo "Mounting boot..." | tee -a ${log}
    MountBoot
    if [ $? -ne 0 ]; then
        echo "Mounting boot failed" | tee -a ${log}
        return 1
    fi

    # download rootfs
    echo "Downloading rootfs..." | tee -a ${log}
    GetRootfs
    if [ $? -ne 0 ]; then
        echo "Downloading rootfs failed" | tee -a ${log}
        return 1
    fi

    # set boot
    echo "Setting boot..." | tee -a ${log}
    SetBoot
    if [ $? -ne 0 ]; then
        echo "Setting boot failed" | tee -a ${log}
        return 1
    fi

    return 0
}

Bootup_Main
ret=$?
if [ ${ret} -eq 0 ]; then
    echo "kubeOS install success! switch to root" | tee -a ${log}
    cp ${log} /persist
else
    echo "kubeOS install failed, see install.log" | tee -a ${log}
fi

