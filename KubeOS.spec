# Copyright (c) Huawei Technologies Co., Ltd. 2021. All rights reserved.

Name:           KubeOS
Version:        1.0.0
Release:        4
Summary:        O&M platform used to update the whole OS as an entirety
License:        Mulan PSL v2
Source0:        https://gitee.com/openeuler/isula-build/repository/archive/v%{version}.tar.gz
ExclusiveArch:  x86_64
BuildRoot:      %{_tmppath}/%{name}-%{version}-build
Patch1: 0001-argumentchange.patch
BuildRequires:  make
BuildRequires:  golang >= 1.13
%description
This is an O&M platform used to update the whole OS as an entirety,
it should be running in kubernetes environment.

%prep
%autosetup -n %{name} -p1

%package scripts
Summary: Scripts to build the os image and binaries of os-proxy and os-operator
Requires: qemu-img, parted, bc, tar, docker
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

install -d -m 0740 %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/boot-grub2.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/etc.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/persist.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/var.mount %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-agent.service %{buildroot}/opt/kubeOS/files
install -p -m 0600 ./files/os-release %{buildroot}/opt/kubeOS/files

%files
%attr(0500,root,root) /opt/kubeOS/bin/os-agent
%defattr(-,root,root,0500)
%attr(0600,root,root) /opt/kubeOS/files/boot-grub2.mount
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

%clean
rm -rfv %{buildroot}

%changelog
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
