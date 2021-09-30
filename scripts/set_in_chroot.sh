#!/bin/bash
ln -s /usr/lib/systemd/system/os-agent.service /usr/lib/systemd/system/multi-user.target.wants/os-agent.service
ln -s /usr/lib/systemd/system/kubelet.service /usr/lib/systemd/system/multi-user.target.wants/kubelet.service
ln -s /usr/lib/systemd/system/boot-grub2.mount /lib/systemd/system/local-fs.target.wants/boot-grub2.mount
ln -s /usr/lib/systemd/system/etc.mount /lib/systemd/system/local-fs.target.wants/etc.mount

str=`sed -n '/^root:/p' /etc/shadow | awk -F "root:" '{print $2}'`
umask 0666
mv /etc/shadow /etc/shadow_bak
sed -i '/^root:/d' /etc/shadow_bak
echo "root:"${ROOT_PWD}${str:1} > /etc/shadow
cat /etc/shadow_bak >> /etc/shadow
rm -rf /etc/shadow_bak
