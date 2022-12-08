# Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.

Name:           KubeOS
Version:        1.0.2
Release:        8
Summary:        O&M platform used to update the whole OS as an entirety
License:        Mulan PSL v2
Source0:        https://gitee.com/openeuler/KubeOS/repository/archive/v%{version}.tar.gz
Patch1:         0001-Write-a-tool-to-support-KubeOS-deployment-on-physica.patch
Patch2:         0002-KubeOS-fix-the-kbimg.sh-exception-and-pxe-installati.patch
Patch3:         0003-KubeOS-fixed-the-issue-of-VMs-images-and-add-check-o.patch
Patch4:         0004-KubeOS-add-the-clearing-of-space-before-the-upgrade-.patch
Patch5:         0005-KubeOS-add-the-configuration-of-etc-resolv.conf-and-.patch
Patch6:         0006-KubeOS-remove-grub2-legacy-install-add-error-handlin.patch
Patch7:         0007-KubeOS-fix-usage-does-not-print-when-an-error-occurs.patch
BuildRoot:      %{_tmppath}/%{name}-%{version}-build
BuildRequires:  make
BuildRequires:  golang >= 1.13
%description
This is an O&M platform used to update the whole OS as an entirety,
it should be running in kubernetes environment.

%prep
%autosetup -n %{name}-v%{version} -p1

%package scripts
Summary: Scripts to build the os image and binaries of os-proxy and os-operator
Requires: qemu-img, parted, bc, tar, docker, dosfstools
%description scripts
The scripts package includes scripts which could build the os image and binaries of os-proxy and os-operator

%define debug_package %{nil}
%define __debug_install_post \
%{_rpmconfigdir}/find-debuginfo.sh %{?_find_debuginfo_opts} "%{_builddir}/%{?buildsubdir}" \
%{nil}

%build
make

%install
install -d %{buildroot}%{_bindir}
#install binary
install -d -m 0740 %{buildroot}/opt/kubeOS/bin
install -p -m 0500 ./bin/os-agent %{buildroot}/opt/kubeOS/bin
install -p -m 0500 ./bin/proxy %{buildroot}/opt/kubeOS/bin
install -p -m 0500 ./bin/operator %{buildroot}/opt/kubeOS/bin

#install artifacts
install -d -m 0740 %{buildroot}/opt/kubeOS/scripts
install -p -m 0600 ./scripts/rpmlist %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/kbimg.sh %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/set_in_chroot.sh %{buildroot}/opt/kubeOS/scripts
install -p -m 0600 ./scripts/grub.cfg %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/bootloader.sh %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/Dockerfile %{buildroot}/opt/kubeOS/scripts

install -d -m 0740 %{buildroot}/opt/kubeOS/scripts/common
install -p -m 0500 ./scripts/common/globalVariables.sh %{buildroot}/opt/kubeOS/scripts/common
install -p -m 0500 ./scripts/common/log.sh %{buildroot}/opt/kubeOS/scripts/common
install -p -m 0500 ./scripts/common/utils.sh %{buildroot}/opt/kubeOS/scripts/common

install -d -m 0740 %{buildroot}/opt/kubeOS/scripts/create
install -p -m 0500 ./scripts/create/imageCreate.sh %{buildroot}/opt/kubeOS/scripts/create
install -p -m 0500 ./scripts/create/rootfsCreate.sh %{buildroot}/opt/kubeOS/scripts/create

install -d -m 0740 %{buildroot}/opt/kubeOS/scripts/00bootup
install -p -m 0600 ./scripts/00bootup/Global.cfg %{buildroot}/opt/kubeOS/scripts/00bootup
install -p -m 0500 ./scripts/00bootup/module-setup.sh %{buildroot}/opt/kubeOS/scripts/00bootup
install -p -m 0500 ./scripts/00bootup/mount.sh %{buildroot}/opt/kubeOS/scripts/00bootup

install -d -m 0740 %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/boot-efi.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/etc.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/persist.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/var.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-agent.service %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-release %{buildroot}/opt/kubeOS/files

%files
%attr(0500,root,root) /opt/kubeOS/bin/os-agent
%defattr(-,root,root,0500)
%attr(0600,root,root) /opt/kubeOS/files/boot-efi.mount
%attr(0600,root,root) /opt/kubeOS/files/etc.mount
%attr(0600,root,root) /opt/kubeOS/files/persist.mount
%attr(0600,root,root) /opt/kubeOS/files/var.mount
%attr(0600,root,root) /opt/kubeOS/files/os-agent.service
%attr(0600,root,root) /opt/kubeOS/files/os-release

%files scripts
%attr(0500,root,root) /opt/kubeOS/bin/proxy
%attr(0500,root,root) /opt/kubeOS/bin/operator
%defattr(-,root,root,0500)
%attr(0600,root,root) /opt/kubeOS/scripts/rpmlist
%attr(0500,root,root) /opt/kubeOS/scripts/kbimg.sh
%attr(0500,root,root) /opt/kubeOS/scripts/set_in_chroot.sh
%attr(0600,root,root) /opt/kubeOS/scripts/grub.cfg
%attr(0500,root,root) /opt/kubeOS/scripts/bootloader.sh
%attr(0500,root,root) /opt/kubeOS/scripts/Dockerfile

%attr(0500,root,root) /opt/kubeOS/scripts/common/globalVariables.sh
%attr(0500,root,root) /opt/kubeOS/scripts/common/log.sh
%attr(0500,root,root) /opt/kubeOS/scripts/common/utils.sh

%attr(0500,root,root) /opt/kubeOS/scripts/create/imageCreate.sh
%attr(0500,root,root) /opt/kubeOS/scripts/create/rootfsCreate.sh

%attr(0600,root,root) /opt/kubeOS/scripts/00bootup/Global.cfg
%attr(0500,root,root) /opt/kubeOS/scripts/00bootup/module-setup.sh
%attr(0500,root,root) /opt/kubeOS/scripts/00bootup/mount.sh


%clean
rm -rfv %{buildroot}

%changelog
* Thu Dec 08 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-8
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fix usage does not print when an error occurs in the upgrade image creation

* Tue Nov 29 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-7
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:remove grub2 legacy install, add error handling for opstype and add entry for unit test in Makefile

* Sat Sep 03 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-6
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:add the configuration of /etc/resolv.conf and change the VM disk to gpt.

* Wed Aug 31 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-5
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:add the clearing of space before the upgrade and rectifying the rollback failure.

* Mon Aug 29 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-4
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fixed the issue of VMs images and add check of Global.cfg.

* Tue Aug 23 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-3
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fix the kbimg.sh exception and pxe installation

* Fri Aug 05 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.2-2
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:update to 1.0.2-2

* Tue Aug 02 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-8
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:update to 1.0.1-8

* Fri Dec 17 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-5
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:add arm architecture support to the OS image

* Wed Dec 08 2021 linxiaoxu<linxiaoxu@huawei.com> - 1.0.1-4
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fix bugs of change generate argument from isopath to repopath

* Thu Nov 11 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-3
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fix bugs of checks in generate.sh and change module path

* Fri Oct 30 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-2
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:update compressed package

* Fri Oct 29 2021 linxiaoxu<linxiaoxu@huawei.com> - 1.0.1-1
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:update version to v1.0.1

* Tue Oct 19 2021 linxiaoxu<linxiaoxu@huawei.com> - 1.0.0-4
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:change argument check range

* Thu Sep 30 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.0-2
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:update spec

* Thu Sep 30 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.0-2
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:add Arch to Spec

* Thu Sep 30 2021 liyuanrong<liyuanrong1@huawei.com> - 1.0.0-1
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:First release KubeOS in rpm package
