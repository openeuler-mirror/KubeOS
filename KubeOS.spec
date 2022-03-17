# Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.

Name:           KubeOS
Version:        1.0.1
Release:        8
Summary:        O&M platform used to update the whole OS as an entirety
License:        Mulan PSL v2
Source0:        https://gitee.com/openeuler/KubeOS/repository/archive/v%{version}.tar.gz
Patch1:         0001-KubeOS-modify-checks-in-generate.sh-and-change-modul.patch
Patch2:         0002-change-generate-argument-from-isopath-to-repopath.patch
Patch3:         0003-KubeOS-add-arm-architecture-support-to-the-OS-image.patch
Patch4:         0004-KubOS-increase-the-space-of-the-boot-partition.patch
BuildRoot:      %{_tmppath}/%{name}-%{version}-build
BuildRequires:  make
BuildRequires:  golang >= 1.13
%description
This is an O&M platform used to update the whole OS as an entirety,
it should be running in kubernetes environment.

%prep
%autosetup -n %{name} -p1

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
install -p -m 0500 ./scripts/generate.sh %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/set_in_chroot.sh %{buildroot}/opt/kubeOS/scripts
install -p -m 0600 ./scripts/grub.cfg %{buildroot}/opt/kubeOS/scripts
install -p -m 0500 ./scripts/bootloader.sh %{buildroot}/opt/kubeOS/scripts

install -d -m 0740 %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/boot.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/etc.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/persist.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/var.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-agent.service %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-release %{buildroot}/opt/kubeOS/files

%files
%attr(0500,root,root) /opt/kubeOS/bin/os-agent
%defattr(-,root,root,0500)
%attr(0600,root,root) /opt/kubeOS/files/boot.mount
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
%attr(0500,root,root) /opt/kubeOS/scripts/generate.sh
%attr(0500,root,root) /opt/kubeOS/scripts/set_in_chroot.sh
%attr(0600,root,root) /opt/kubeOS/scripts/grub.cfg
%attr(0500,root,root) /opt/kubeOS/scripts/bootloader.sh

%clean
rm -rfv %{buildroot}

%changelog
* Fri Mar 11 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-8
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:Undo the temporary fix for the sshd startup failure

* Wed Mar 02 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-7
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:fix sshd startup failed

* Wed Mar 02 2022 liyuanrong<liyuanrong1@huawei.com> - 1.0.1-6
- Type:requirement
- CVE:NA
- SUG:restart
- DESC:increase the space of the boot partition

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
