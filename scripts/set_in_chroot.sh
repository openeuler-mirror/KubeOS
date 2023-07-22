#!/bin/bash
ln -s /usr/lib/systemd/system/os-agent.service /usr/lib/systemd/system/multi-user.target.wants/os-agent.service
ln -s /usr/lib/systemd/system/kubelet.service /usr/lib/systemd/system/multi-user.target.wants/kubelet.service
if [ "$BOOT_MODE" = "legacy" ]; then
    ln -s /usr/lib/systemd/system/boot-grub2.mount /lib/systemd/system/local-fs.target.wants/boot-grub2.mount
else
    ln -s /usr/lib/systemd/system/boot-efi.mount /lib/systemd/system/local-fs.target.wants/boot-efi.mount
fi

str=`sed -n '/^root:/p' /etc/shadow | awk -F "root:" '{print $2}'`
umask 0666
mv /etc/shadow /etc/shadow_bak
sed -i '/^root:/d' /etc/shadow_bak
echo "root:"${ROOT_PWD}${str:1} > /etc/shadow
cat /etc/shadow_bak >> /etc/shadow
rm -rf /etc/shadow_bak

# move the 10-mount-etc.sh out
mv /usr/lib/dracut/modules.d/00bootup/10-mount-etc.sh /

# make initramfs.img for baremetal PXE mode
dracut -f -v --add bootup /initramfs.img --include /10-mount-etc.sh /usr/lib/dracut/hooks/pre-pivot/10-mount-etc.sh --kver `ls /lib/modules`

# make initramfs.img for vms
rm -f /boot/initramfs.img
dracut -f -v /boot/initramfs.img --include /10-mount-etc.sh /usr/lib/dracut/hooks/pre-pivot/10-mount-etc.sh --force --kver `ls /lib/modules`
rm -rf /usr/lib/dracut/modules.d/00bootup /10-mount-etc.sh