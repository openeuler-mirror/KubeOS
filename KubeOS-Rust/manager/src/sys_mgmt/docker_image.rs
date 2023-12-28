use anyhow::Result;
use log::{debug, info};

use crate::{
    api::{ImageHandler, UpgradeRequest},
    sys_mgmt::{IMAGE_PERMISSION, NEED_BYTES, PERSIST_DIR},
    utils::*,
};

pub struct DockerImageHandler<T: CommandExecutor> {
    pub paths: PreparePath,
    pub container_name: String,
    pub executor: T,
}

impl<T: CommandExecutor> ImageHandler<T> for DockerImageHandler<T> {
    fn download_image(&self, req: &UpgradeRequest) -> Result<UpgradeImageManager<T>> {
        perpare_env(&self.paths, NEED_BYTES, PERSIST_DIR, IMAGE_PERMISSION)?;
        self.get_image(req)?;
        self.get_rootfs_archive(req)?;

        let (_, next_partition_info) = get_partition_info(&self.executor)?;
        let img_manager = UpgradeImageManager::new(
            self.paths.clone(),
            next_partition_info,
            self.executor.clone(),
        );
        img_manager.create_os_image(IMAGE_PERMISSION)
    }
}

impl Default for DockerImageHandler<RealCommandExecutor> {
    fn default() -> Self {
        Self {
            paths: PreparePath::default(),
            container_name: "kubeos-temp".into(),
            executor: RealCommandExecutor {},
        }
    }
}

impl<T: CommandExecutor> DockerImageHandler<T> {
    #[cfg(test)]
    fn new(paths: PreparePath, container_name: String, executor: T) -> Self {
        Self {
            paths,
            container_name,
            executor,
        }
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
        let container_id = self.executor.run_command_with_output(
            "docker",
            &["create", "--name", &self.container_name, image_name],
        )?;
        debug!(
            "Copy rootfs from container {} to {}",
            container_id,
            self.paths.update_path.display()
        );
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
        let docker_ps_cmd = format!(
            "docker ps -a -f=name={} | awk 'NR==2' | awk '{{print $1}}'",
            self.container_name
        );
        let exist_id = self
            .executor
            .run_command_with_output("bash", &["-c", &docker_ps_cmd])?;
        if !exist_id.is_empty() {
            info!(
                "Remove container {} {} for cleaning environment",
                self.container_name, exist_id
            );
            self.executor
                .run_command("docker", &["rm", exist_id.as_str()])?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;

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

        let result = DockerImageHandler::new(PreparePath::default(), "test".into(), mock_executor)
            .check_and_rm_container();
        assert!(result.is_ok());
    }
}
