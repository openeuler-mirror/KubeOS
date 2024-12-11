/*
 * Copyright (c) Huawei Technologies Co., Ltd. 2024. All rights reserved.
 * KubeOS is licensed under the Mulan PSL v2.
 * You can use this software according to the terms and conditions of the Mulan PSL v2.
 * You may obtain a copy of Mulan PSL v2 at:
 *     http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND, EITHER EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR
 * PURPOSE.
 * See the Mulan PSL v2 for more details.
 */

use std::{
    fs::{self, File},
    io::Write,
};

use anyhow::{anyhow, Result};

use crate::{
    commands::{ChrootScript, CopyFile, DmVerity, Grub, User},
    scripts_gen::base_gen,
    utils,
    values::*,
};

impl CopyFile {
    pub(crate) fn gen_copy_files(&self, file: &mut dyn Write) -> Result<()> {
        if let Some(dir) = &self.create_dir {
            writeln!(file, "    mkdir -p \"${{RPM_ROOT}}\"{}", dir)?;
        }
        if !self.src.is_empty() && !self.dst.is_empty() {
            writeln!(file, "    cp -r {} \"${{RPM_ROOT}}{}\"", self.src, self.dst)?;
        }
        Ok(())
    }
}

impl Grub {
    pub(crate) fn gen_grub_config(&self, file: &mut dyn Write, legacy_bios: bool, arch: &str) -> Result<()> {
        writeln!(
            file,
            r#"function grub_config() {{
    local GRUB_PATH"#
        )?;
        if legacy_bios && arch == "x86_64" {
            writeln!(file, "    GRUB_PATH=\"${{RPM_ROOT}}\"/boot/grub2")?;
        } else {
            writeln!(file, "    GRUB_PATH=\"${{RPM_ROOT}}\"/boot/efi/EFI/openEuler")?;
        }
        writeln!(
            file,
            r#"    local GRUB_PASSWORD
    GRUB_PASSWORD=$(echo -e "{}\n{}" | grub2-mkpasswd-pbkdf2 | grep PBKDF2 | awk '{{print $7}}')
    echo "GRUB2_PASSWORD=${{GRUB_PASSWORD}}" >"${{GRUB_PATH}}"/user.cfg
    chmod 600 "${{GRUB_PATH}}"/user.cfg
}}
"#,
            self.passwd, self.passwd
        )?;
        Ok(())
    }
}

impl ChrootScript {
    pub(crate) fn gen_chroot_script(&self, file: &mut dyn Write) -> Result<()> {
        let script_path = &self.path;
        utils::is_file_valid("chroot script", &script_path)?;
        let absolute_path = script_path.canonicalize()?;
        let script_name = absolute_path.file_name().ok_or_else(|| anyhow!("script name not found"))?;
        writeln!(
            file,
            r#"function chroot_script() {{
    cp "{}" "${{RPM_ROOT}}"
    chroot "${{RPM_ROOT}}" bash /{}
    {}
}}
"#,
            absolute_path.as_path().to_str().unwrap(),
            script_name.to_str().unwrap(),
            if self.rm.unwrap_or(false) {
                format!("rm -f \"${{RPM_ROOT}}\"/{}", script_name.to_str().unwrap())
            } else {
                "".to_string()
            }
        )?;
        Ok(())
    }
}

impl User {
    pub(crate) fn gen_add_users(&self, file: &mut dyn Write) -> Result<()> {
        let name = &self.name;
        let passwd = &self.passwd;
        let mut group_script = String::new();
        let mut script = format!("useradd -m");
        group_script.push_str(&format!("getent group {} || groupadd {}", name, name));
        if let Some(primary_group) = &self.primary_group {
            script.push_str(&format!(" -g {}", primary_group));
        } else {
            script.push_str(&format!(" -g {}", name));
        }
        if let Some(groups) = &self.groups {
            script.push_str(&format!(" -G {}", groups.join(",")));
        }
        script.push_str(&format!(" -s /bin/bash \"{}\"\n", name));
        script.push_str(&format!("echo \"{}:{}\" | chpasswd", name, passwd));
        writeln!(file, "{}\n{}", group_script, script)?;
        Ok(())
    }
}

impl DmVerity {
    pub(crate) fn write_dm_verity_repo(&self) -> Result<()> {
        fs::create_dir_all(DMV_DIR)?;
        utils::set_permissions(DMV_DIR, DIR_PERMISSION)?;
        let dmv_chroot = format!("{}/{}", DMV_DIR, DMV_CHROOT);
        let dmv_main = format!("{}/{}", DMV_DIR, DMV_MAIN);
        let dmv_upgrade_rollback = format!("{}/{}", DMV_DIR, DMV_UPGRADE_ROLLBACK);
        let mut dmv_main_file = File::create(&dmv_main)?;
        let mut dmv_chroot_file = File::create(&dmv_chroot)?;
        let mut dmv_upgrade_rollback_file = File::create(&dmv_upgrade_rollback)?;
        base_gen(&mut dmv_main_file, DMV_MAIN_SH, true)?;
        base_gen(&mut dmv_chroot_file, DMV_CHROOT_NEW_GRUB_SH, true)?;
        base_gen(&mut dmv_upgrade_rollback_file, DMV_UPGRADE_ROLLBACK_SH, true)?;
        utils::set_permissions(&dmv_main, EXEC_PERMISSION)?;
        utils::set_permissions(&dmv_chroot, EXEC_PERMISSION)?;
        utils::set_permissions(&dmv_upgrade_rollback, EXEC_PERMISSION)?;

        fs::create_dir_all(DMV_DRACUT_DIR)?;
        utils::set_permissions(DMV_DRACUT_DIR, DIR_PERMISSION)?;
        let dmv_dracut_module = format!("{}/{}", DMV_DRACUT_DIR, DMV_DRACUT_MODULE);
        let dmv_dracut_mount = format!("{}/{}", DMV_DRACUT_DIR, DMV_DRACUT_MOUNT);
        let mut dmv_dracut_module_file = File::create(&dmv_dracut_module)?;
        let mut dmv_dracut_mount_file = File::create(&dmv_dracut_mount)?;
        base_gen(&mut dmv_dracut_module_file, DMV_MODULE_SETUP_SH, true)?;
        base_gen(&mut dmv_dracut_mount_file, DMV_MOUNT_SH, true)?;
        utils::set_permissions(&dmv_dracut_module, EXEC_PERMISSION)?;
        utils::set_permissions(&dmv_dracut_mount, EXEC_PERMISSION)?;
        Ok(())
    }

    pub(crate) fn write_dm_verity_upgrade(&self, file: &mut dyn Write) -> Result<()> {
        self.write_dm_verity_repo()?;
        self.write_dmv_dockerignore()?;
        self.write_dmv_upgrade_dockerfile()?;

        writeln!(file, r#"docker build -t "${{DOCKER_IMG}}" -f "${{SCRIPTS_DIR}}"/Dockerfile "${{SCRIPTS_DIR}}""#)?;
        Ok(())
    }

    pub(crate) fn write_dmv_upgrade_dockerfile(&self) -> Result<()> {
        let dockerfile_path = format!("{}/{}", SCRIPTS_DIR, DOCKERFILE);
        let mut dockerfile = File::create(&dockerfile_path)?;
        base_gen(&mut dockerfile, DMV_DOCKERFILE, false)?;
        utils::set_permissions(&dockerfile_path, CONFIG_PERMISSION)?;
        Ok(())
    }

    pub(crate) fn write_dmv_dockerignore(&self) -> Result<()> {
        let dockerig_path = format!("{}/{}", SCRIPTS_DIR, ".dockerignore");
        let mut dockerignore = File::create(&dockerig_path)?;
        base_gen(&mut dockerignore, "system.*", false)?;
        utils::set_permissions(&dockerig_path, CONFIG_PERMISSION)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_copy_files_with_multiple_dirs() {
        let copy_file = CopyFile {
            src: String::from("/home/aaa/test.txt"),
            dst: String::from("/home/bbb/test/test1.txt"),
            create_dir: Some(String::from("/home/bbb/test")),
        };

        let mut buffer = std::io::Cursor::new(Vec::new());
        copy_file.gen_copy_files(&mut buffer).unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        println!("{}", output);
        assert_eq!(
            output,
            "    mkdir -p \"${RPM_ROOT}\"/home/bbb/test\n    cp -r /home/aaa/test.txt \"${RPM_ROOT}/home/bbb/test/test1.txt\"\n"
        );
    }

    #[test]
    fn test_gen_users() {
        let user = User {
            name: String::from("test"),
            passwd: String::from("test"),
            primary_group: Some(String::from("test")),
            groups: Some(vec![String::from("test1"), String::from("test2")]),
        };

        let mut buffer = std::io::Cursor::new(Vec::new());
        user.gen_add_users(&mut buffer).unwrap();

        let output = String::from_utf8(buffer.into_inner()).unwrap();
        println!("{}", output);
        assert_eq!(
            output,
            "getent group test || groupadd test\nuseradd -m -g test -G test1,test2 -s /bin/bash \"test\"\necho \"test:test\" | chpasswd\n"
        );
    }
}
