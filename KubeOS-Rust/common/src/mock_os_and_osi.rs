use std::collections::BTreeMap;

use kube::{api::ObjectMeta, Resource};

use crate::{
    crd::{Config, Configs, Content, NamespacedName, OSInstance, OSInstanceSpec, OSInstanceStatus, OSSpec, OS},
    values::{LABEL_OSINSTANCE, NODE_STATUS_CONFIG, NODE_STATUS_IDLE, NODE_STATUS_UPGRADE, OPERATION_TYPE_CONFIG, OPERATION_TYPE_ROLLBACK },
};

impl OSInstance {
    pub fn set_osi_default(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = idle, upgradeconfig.version=v1, sysconfig.version=v1
        let mut labels = BTreeMap::new();
        labels.insert(LABEL_OSINSTANCE.to_string(), node_name.to_string());
        OSInstance {
            metadata: ObjectMeta {
                name: Some(node_name.to_string()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            spec: OSInstanceSpec {
                nodestatus: NODE_STATUS_IDLE.to_string(),
                sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                namespacedname:Some(NamespacedName{namespace:String::from("default"),name:String::from("test")})
            },
            status: Some(OSInstanceStatus {
                sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
                upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            }),
        }
    }

    pub fn set_osi_nodestatus_upgrade(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        osinstance
    }

    pub fn set_osi_nodestatus_config(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = config, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
        osinstance
    }

    pub fn set_osi_upgradecon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = idle, upgradeconfig.version=v1, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.upgradeconfigs.as_mut().unwrap().version = Some(String::from("v2"));
        osinstance
    }

    pub fn set_osi_nodestatus_upgrade_upgradecon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v2, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_UPGRADE.to_string();
        osinstance.spec.upgradeconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(String::from("test")),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        osinstance
    }

    pub fn set_osi_nodestatus_config_syscon_v2(node_name: &str, namespace: &str) -> Self {
        // return osinstance with nodestatus = upgrade, upgradeconfig.version=v2, sysconfig.version=v1
        let mut osinstance = OSInstance::set_osi_default(node_name, namespace);
        osinstance.spec.nodestatus = NODE_STATUS_CONFIG.to_string();
        osinstance.spec.sysconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(String::from("test")),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        osinstance
    }

}

impl OS {
    pub fn set_os_default() -> Self {
        let mut os = OS::new("test", OSSpec::default());
        os.meta_mut().namespace = Some("default".into());
        os
    }

    pub fn set_os_osversion_v2_opstype_config() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.opstype = String::from("config");
        os
    }

    pub fn set_os_osversion_v2_upgradecon_v2() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.upgradeconfigs = Some(Configs { version: Some(String::from("v2")), configs: None });
        os
    }

    pub fn set_os_rollback_osversion_v1_upgradecon_v1() -> Self {
        let mut os = OS::set_os_default();
        os.spec.opstype = OPERATION_TYPE_ROLLBACK.to_string();
        os
    }
    
    pub fn set_os_rollback_osversion_v2_upgradecon_v2() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os.spec.opstype = OPERATION_TYPE_ROLLBACK.to_string();
        os.spec.upgradeconfigs = Some(Configs {
            version: Some(String::from("v2")),
            configs: Some(vec![Config {
                model: Some(String::from("kernel.sysctl.persist")),
                configpath: Some(String::from("/persist/persist.conf")),
                contents: Some(vec![Content {
                    key: Some(String::from("kernel.test")),
                    value: Some(String::from("test")),
                    operation: Some(String::from("delete")),
                }]),
            }]),
        });
        os
    }

    pub fn set_os_syscon_v2_opstype_config_operator() -> Self {
        let mut os = OS::set_os_default();
        os.spec.opstype = OPERATION_TYPE_CONFIG.to_string();
        os.spec.sysconfigs = Some(
            Configs {
                version: Some(String::from("v2")),
                configs: Some(vec![Config {
                    model: Some(String::from("kernel.sysctl")),
                    configpath: Some(String::from("")),
                    contents: Some(vec![
                        Content {
                            key: Some(String::from("key1")),
                            value: Some(String::from("a")),
                            operation: Some(String::from("")),
                        }, 
                        Content {
                            key: Some(String::from("key2")),
                            value: Some(String::from("b")),
                            operation: Some(String::from("")),
                        },
                    ]),
                }]),
            }
        );
        os
    }

    pub fn set_os_syscon_v2_opstype_config_proxy() -> Self {
        let mut os = OS::set_os_default();
        os.spec.opstype = String::from("config");
        os.spec.sysconfigs = Some(
            Configs {
                version: Some(String::from("v2")),
                configs: Some(vec![Config {
                    model: Some(String::from("kernel.sysctl.persist")),
                    configpath: Some(String::from("/persist/persist.conf")),
                    contents: Some(vec![Content {
                        key: Some(String::from("kernel.test")),
                        value: Some(String::from("test")),
                        operation: Some(String::from("delete")),
                    }]),
                }]),
            });
        os
    }

    pub fn set_os_skip_osversion_v2_upgradecon_v1() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        os
    }

    pub fn set_os_exchange_current_and_next() -> Self {
        let mut os = OS::set_os_default();
        os.spec.osversion = String::from("KubeOS v2");
        let sysconfigs = Some(
            Configs{
                version: Some(String::from("v2")),
                configs: Some(vec![
                    Config {
                        model: Some(String::from("grub.cmdline.current")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("a")),
                                value: Some(String::from("1")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                    Config {
                        model: Some(String::from("grub.cmdline.next")),
                        configpath: Some(String::from("")),
                        contents: Some(vec![
                            Content {
                                key: Some(String::from("b")),
                                value: Some(String::from("2")),
                                operation: Some(String::from("")),
                            }
                        ]),
                    },
                ]),
            }
        );
        os.spec.sysconfigs = sysconfigs.clone();
        os.spec.upgradeconfigs = sysconfigs.clone();
        os
    }

}

impl Default for OSSpec {
    fn default() -> Self {
        OSSpec {
            osversion: String::from("KubeOS v1"),
            maxunavailable: 2,
            checksum: String::from("test"),
            imagetype: String::from("containerd"),
            containerimage: String::from("test"),
            opstype: String::from("upgrade"),
            evictpodforce: true,
            imageurl: String::from(""),
            flagsafe: false,
            mtls: false,
            cacert: Some(String::from("")),
            clientcert: Some(String::from("")),
            clientkey: Some(String::from("")),
            sysconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            upgradeconfigs: Some(Configs { version: Some(String::from("v1")), configs: None }),
            nodeselector:None,
            timeinterval:None,
            timewindow:None,
            executionmode:None,
        }
    }
}