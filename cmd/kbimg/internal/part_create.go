/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

// Package internal implements the scripts and invocation of KubeOS image customization.
package internal

import (
	"fmt"
	"path/filepath"
	"strconv"

	_ "github.com/mitchellh/mapstructure"
)

// path of partCreate.sh
var partitionScripts = filepath.Join("scripts", "create", "partCreate.sh")

const globalVariable = `#!/bin/bash
	TMP_MOUNT_PATH="${PWD}/mnt"
	RPM_ROOT="${PWD}/rootfs"
	IMG_SIZE=20
	PWD="$(pwd)"`

const createImage = `
	local BOOT_MODE=$1
	rm -f system.img update.img
	qemu-img create system.img "${IMG_SIZE}G"`

const initBoot = `
	if [ "$BOOT_MODE" = "legacy" ]; then
		init_boot_part system.img1 GRUB2 "${BOOT_PATH}"
 	 else
		init_boot_part system.img1 BOOT "${BOOT_PATH}"
  	fi`

// bootloader.sh installs different GRUB2 bootloaders based on different architectures
const initGrub = `
	tar -x -C ${TMP_MOUNT_PATH} -f os.tar
	if [ "$BOOT_MODE" = "legacy" ]; then
		sed -i "s/insmod part_gpt/insmod part_msdos/g; \
		s/set root='hd0,gpt2'/set root='hd0,msdos2'/g; \
		s/set root='hd0,gpt3'/set root='hd0,msdos3'/g" \
		"${TMP_MOUNT_PATH}"/boot/grub2/grub.cfg
 	fi
 	sync`

const downloadGrub = `
	cp bootloader.sh "${TMP_MOUNT_PATH}"
	mount_proc_dev_sys "${TMP_MOUNT_PATH}"
	DEVICE="${device}" BOOT_MODE="${BOOT_MODE}" chroot "${TMP_MOUNT_PATH}" bash bootloader.sh
	rm -rf "${TMP_MOUNT_PATH}/bootloader.sh"
	sync

	dd if=/dev/disk/by-label/ROOT-A of=update.img bs=8M
	sync
	unmount_dir "${TMP_MOUNT_PATH}"`

const finishLoop = `
	losetup -D
	qemu-img convert system.img -O qcow2 system.qcow2`

func applyPartitionScriptLegacy() error {
	content := generatePartScriptLegacy(config.Partitions)

	err := writeFile(partitionScripts, content, ownerPermission)
	if err != nil {
		defer deleteFile(partitionScripts)
		return fmt.Errorf("writing partCreate.sh error: %s", err)
	}

	return nil
}

func applyPartitionScriptEfi() error {
	content := generatePartScriptEfi(config.Partitions)

	err := writeFile(partitionScripts, content, ownerPermission)
	if err != nil {
		defer deleteFile(partitionScripts)
		return fmt.Errorf("writing partCreate.sh error: %s", err)
	}

	return nil
}

func generatePartScriptLegacy(partitions []PartitionConfig) string {

	// create boot partition、set limit and partition type
	createBoot := `
	if [ "$BOOT_MODE" = "legacy" ]; then
		local BOOT_PATH=${TMP_MOUNT_PATH}/boot/grub2
		parted system.img -s mklabel msdos
		parted system.img -s mkpart primary 1MiB ` + strconv.Itoa(config.Partitions[0].Limit) + `MiB
	else
		local BOOT_PATH=${TMP_MOUNT_PATH}/boot/efi
		parted system.img -s mklabel gpt
		parted system.img -s mkpart primary 1MiB ` + strconv.Itoa(config.Partitions[0].Limit) + `MiB
	fi
	`
	// create ROOT-A、ROOT-B primary partition、set limit and partition type
	var partition string
	var persent string = "100%"
	var length int = len(config.Partitions)
	for i := 1; i < 3; i++ {
		partition += fmt.Sprintf("parted system.img -s mkpart primary %dMiB %dMiB\n", config.Partitions[i-1].Limit, config.Partitions[i].Limit)
	}
	// create extended partition，and create logical partition in it
	partition += fmt.Sprintf("parted system.img -s mkpart extended %dMiB %s\n", config.Partitions[2].Limit, persent)
	for i := 3; i < length-1; i++ {
		partition += fmt.Sprintf("parted system.img -s mkpart logical %dMiB %dMiB -l\n", config.Partitions[i-1].Limit, config.Partitions[i].Limit)
	}
	partition += fmt.Sprintf("parted system.img -s mkpart logical %dMiB %s -l\n", config.Partitions[length-2].Limit, persent)
	partition += fmt.Sprintf("parted system.img -s set 1 boot on")

	createLoop := `  
	local device=$(losetup -f)
	losetup "${device}" system.img
	mkdir -p "${TMP_MOUNT_PATH}"

	init_flexible_part system.img2 ROOT-A "${TMP_MOUNT_PATH}" ` + config.Partitions[1].Type + `
	mkdir -p ${BOOT_PATH}
	chmod 755 ${BOOT_PATH}
	`

	// init partitions
	var initPartitions string
	initPartitions += `
	mkdir -p "${TMP_MOUNT_PATH}"
	init_flexible_part system.img3 ROOT-B "${TMP_MOUNT_PATH}" ` + config.Partitions[2].Type + `
	umount "${TMP_MOUNT_PATH}"
	`

	// start from the first parition that uers created
	for i := 3; i < length-1; i++ {
		initPartitions += `
		mkdir -p "${TMP_MOUNT_PATH}"
		init_flexible_part system.img` + strconv.Itoa(i+2) + " " + config.Partitions[i].Label + ` "${TMP_MOUNT_PATH}" ` + config.Partitions[i].Type + `
		umount "${TMP_MOUNT_PATH}"
		`
	}
	initPartitions += `
		init_flexible_part system.img` + strconv.Itoa(length+1) + ` PERSIST "${TMP_MOUNT_PATH}" ` + config.Partitions[length-1].Type + `
		mkdir ${TMP_MOUNT_PATH}/{var,etc,etcwork}
		mkdir -p ${TMP_MOUNT_PATH}/etc/KubeOS/certs
		umount "${TMP_MOUNT_PATH}"
		`

	content := globalVariable + "\n"
	content += "function create_img() {\n"
	content += createImage
	content += createBoot
	content += partition
	content += createLoop
	content += initBoot
	content += initGrub
	content += downloadGrub
	content += initPartitions
	content += finishLoop + "\n"
	content += "}\n"

	return content
}

func generatePartScriptEfi(partitions []PartitionConfig) string {

	// create boot partition、set limit and partition type
	createBoot := `
	if [ "$BOOT_MODE" = "legacy" ]; then
		local BOOT_PATH=${TMP_MOUNT_PATH}/boot/grub2
		parted system.img -s mklabel msdos
		parted system.img -s mkpart primary 1MiB ` + strconv.Itoa(config.Partitions[0].Limit) + `MiB
	else
		local BOOT_PATH=${TMP_MOUNT_PATH}/boot/efi
		parted system.img -s mklabel gpt
		parted system.img -s mkpart primary 1MiB ` + strconv.Itoa(config.Partitions[0].Limit) + `MiB
	fi
	`
	// create ROOT-A、ROOT-B、user config partitions、primary partition, set limit and partition type
	var partition string
	var persent string = "100%"
	var length int = len(config.Partitions)
	for i := 1; i < length-1; i++ {
		partition += fmt.Sprintf("parted system.img -s mkpart primary %dMiB %dMiB\n", config.Partitions[i-1].Limit, config.Partitions[i].Limit)
	}
	partition += fmt.Sprintf("parted system.img -s mkpart primary %dMiB %s\n", config.Partitions[length-2].Limit, persent)
	partition += fmt.Sprintf("parted system.img -s set 1 boot on")

	createLoop := `  
	local device=$(losetup -f)
	losetup "${device}" system.img

	mkdir -p "${TMP_MOUNT_PATH}"

	init_flexible_part system.img2 ROOT-A "${TMP_MOUNT_PATH}" ` + config.Partitions[1].Type + `

	mkdir -p ${BOOT_PATH}
	chmod 755 ${BOOT_PATH}
	`

	// init partition from ROOT-B
	var initPartitions string
	for i := 2; i < length-1; i++ {
		initPartitions += `
		mkdir -p "${TMP_MOUNT_PATH}"
		init_flexible_part system.img` + strconv.Itoa(i+1) + " " + config.Partitions[i].Label + ` "${TMP_MOUNT_PATH}" ` + config.Partitions[i].Type + `
		umount "${TMP_MOUNT_PATH}"
		`
	}
	initPartitions += `
		init_flexible_part system.img` + strconv.Itoa(length) + ` PERSIST "${TMP_MOUNT_PATH}" ` + config.Partitions[length-1].Type + `
		mkdir ${TMP_MOUNT_PATH}/{var,etc,etcwork}
		mkdir -p ${TMP_MOUNT_PATH}/etc/KubeOS/certs
		umount "${TMP_MOUNT_PATH}"
		`

	content := globalVariable + "\n"
	content += "function create_img() {\n"
	content += createImage
	content += createBoot
	content += partition
	content += createLoop
	content += initBoot
	content += initGrub
	content += downloadGrub
	content += initPartitions
	content += finishLoop + "\n"
	content += "}\n"

	return content
}
