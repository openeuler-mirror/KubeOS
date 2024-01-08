use anyhow::Result;
use log::{debug, info, trace};

use crate::{
    api::{ImageHandler, UpgradeRequest},
    sys_mgmt::{IMAGE_PERMISSION, NEED_BYTES},
    utils::*,
};

pub struct DockerImageHandler<T: CommandExecutor> {
    pub paths: PreparePath,
    pub container_name: String,
    pub executor: T,
}

impl<T: CommandExecutor> ImageHandler<T> for DockerImageHandler<T> {
    fn download_image(&self, req: &UpgradeRequest) -> Result<UpgradeImageManager<T>> {
        perpare_env(&self.paths, NEED_BYTES, IMAGE_PERMISSION)?;
        self.get_image(req)?;
        self.get_rootfs_archive(req)?;

        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager = UpgradeImageManager::new(self.paths.clone(), next_partition_info, self.executor.clone());
        img_manager.create_os_image(IMAGE_PERMISSION)
    }
}

impl Default for DockerImageHandler<RealCommandExecutor> {
    fn default() -> Self {
        Self { paths: PreparePath::default(), container_name: "kubeos-temp".into(), executor: RealCommandExecutor {} }
    }
}

impl<T: CommandExecutor> DockerImageHandler<T> {
    #[cfg(test)]
    fn new(paths: PreparePath, container_name: String, executor: T) -> Self {
        Self { paths, container_name, executor }
    }

    fn get_image(&self, req: &UpgradeRequest) -> Result<()> {
        let image_name = &req.container_image;
        is_valid_image_name(image_name)?;
        let cli = "docker";
        remove_image_if_exist(cli, image_name, &self.executor)?;
        info!("Start pull image {}", image_name);
        pull_image(cli, image_name, &self.executor)?;
        info!("Start check image digest");
        check_oci_image_digest(cli, image_name, &req.check_sum, &self.executor)?;
        Ok(())
    }

    fn get_rootfs_archive(&self, req: &UpgradeRequest) -> Result<()> {
        let image_name = &req.container_image;
        info!("Start get rootfs {}", image_name);
        self.check_and_rm_container()?;
        debug!("Create container {}", self.container_name);
        let container_id =
            self.executor.run_command_with_output("docker", &["create", "--name", &self.container_name, image_name])?;
        debug!("Copy rootfs from container {} to {}", container_id, self.paths.update_path.display());
        self.executor.run_command(
            "docker",
            &[
                "cp",
                format!("{}:/{}", container_id, self.paths.rootfs_file).as_str(),
                self.paths.update_path.to_str().unwrap(),
            ],
        )?;
        self.check_and_rm_container()?;
        Ok(())
    }

    fn check_and_rm_container(&self) -> Result<()> {
        trace!("Check and remove container {}", self.container_name);
        let docker_ps_cmd = format!("docker ps -a -f=name={} | awk 'NR==2' | awk '{{print $1}}'", self.container_name);
        let exist_id = self.executor.run_command_with_output("bash", &["-c", &docker_ps_cmd])?;
        if !exist_id.is_empty() {
            info!("Remove container {} {} for cleaning environment", self.container_name, exist_id);
            self.executor.run_command("docker", &["rm", exist_id.as_str()])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mockall::mock;

    use super::*;
    use crate::api::CertsInfo;

    mock! {
        pub CommandExec{}
        impl CommandExecutor for CommandExec {
            fn run_command<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<()>;
            fn run_command_with_output<'a>(&self, name: &'a str, args: &[&'a str]) -> Result<String>;
        }
        impl Clone for CommandExec {
            fn clone(&self) -> Self;
        }
    }

    fn init() {
        let _ = env_logger::builder()
            .target(env_logger::Target::Stdout)
            .filter_level(log::LevelFilter::Trace)
            .is_test(true)
            .try_init();
    }

    #[test]
    fn test_check_and_rm_container() {
        init();
        let mut mock_executor = MockCommandExec::new();
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| {
                cmd == "bash"
                    && args.len() == 2
                    && args.contains(&"docker ps -a -f=name=test | awk 'NR==2' | awk '{print $1}'")
            })
            .times(1)
            .returning(|_, _| Ok(String::from("1111")));
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"rm") && args.contains(&"1111"))
            .times(1)
            .returning(|_, _| Ok(()));

        let result =
            DockerImageHandler::new(PreparePath::default(), "test".into(), mock_executor).check_and_rm_container();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_image() {
        init();
        let mut mock_executor = MockCommandExec::new();
        let image_name = "docker.io/library/busybox:latest";
        let req = UpgradeRequest {
            version: "KubeOS v2".to_string(),
            image_type: "docker".to_string(),
            container_image: image_name.to_string(),
            check_sum: "22222".to_string(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };

        // mock remove_image_if_exist
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"inspect")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"rmi")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(()));
        // mock pull_image
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| {
                cmd == "docker" && args.contains(&"pull") && args.contains(&"docker.io/library/busybox:latest")
            })
            .times(1)
            .returning(|_, _| Ok(()));
        // mock get_oci_image_digest
        let command_output2 = "[docker.io/library/busybox:latest@sha256:22222]";
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"inspect") && args.contains(&"{{.RepoDigests}}"))
            .times(1)
            .returning(|_, _| Ok(command_output2.to_string()));

        let docker = DockerImageHandler::new(PreparePath::default(), "kubeos-temp".into(), mock_executor);
        let result = docker.get_image(&req);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_rootfs_archive() {
        init();
        let mut mock_executor = MockCommandExec::new();
        let image_name = "docker.io/library/busybox:latest";
        let req = UpgradeRequest {
            version: "KubeOS v2".to_string(),
            image_type: "docker".to_string(),
            container_image: image_name.to_string(),
            check_sum: "22222".to_string(),
            image_url: "".to_string(),
            flag_safe: false,
            mtls: false,
            certs: CertsInfo { ca_cert: "".to_string(), client_cert: "".to_string(), client_key: "".to_string() },
        };
        // mock check_and_rm_container
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| {
                cmd == "bash" && args.contains(&"docker ps -a -f=name=kubeos-temp | awk 'NR==2' | awk '{print $1}'")
            }) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(String::new()));
        // mock get_rootfs_archive
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"create")) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(String::from("1111")));
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"cp") && args.contains(&"1111:/os.tar"))
            .times(1)
            .returning(|_, _| Ok(()));
        // mock check_and_rm_container
        mock_executor
            .expect_run_command_with_output()
            .withf(|cmd, args| {
                cmd == "bash" && args.contains(&"docker ps -a -f=name=kubeos-temp | awk 'NR==2' | awk '{print $1}'")
            }) // simplified with a closure
            .times(1)
            .returning(|_, _| Ok(String::from("1111")));
        mock_executor
            .expect_run_command()
            .withf(|cmd, args| cmd == "docker" && args.contains(&"rm") && args.contains(&"1111"))
            .times(1)
            .returning(|_, _| Ok(()));

        let docker = DockerImageHandler::new(PreparePath::default(), "kubeos-temp".into(), mock_executor);
        let result = docker.get_rootfs_archive(&req);
        assert!(result.is_ok());
    }
}
