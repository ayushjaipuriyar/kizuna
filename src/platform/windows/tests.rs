// Tests for Windows platform implementation

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_windows_adapter_creation() {
        let adapter = WindowsAdapter::new();
        assert_eq!(adapter.platform_name(), "windows");
    }

    #[test]
    fn test_architecture_detection() {
        #[cfg(windows)]
        {
            let result = architecture::WindowsArchitecture::detect();
            assert!(result.is_ok());
            let arch = result.unwrap();
            assert!(matches!(
                arch,
                architecture::WindowsArchitecture::X64
                    | architecture::WindowsArchitecture::ARM64
                    | architecture::WindowsArchitecture::X86
                    | architecture::WindowsArchitecture::Unknown
            ));
        }
    }

    #[test]
    fn test_architecture_optimizer() {
        let optimizer = architecture::ArchitectureOptimizer::default();
        let config = optimizer.apply_optimizations();
        assert!(config.is_ok());
    }

    #[test]
    fn test_registry_manager_creation() {
        let registry = registry::RegistryManager::new();
        assert_eq!(registry.app_key_path, "Software\\Kizuna");
    }

    #[test]
    fn test_networking_manager_creation() {
        let networking = networking::WindowsNetworking::new();
        // Just verify it can be created
        drop(networking);
    }

    #[test]
    fn test_installer_manager_creation() {
        let installer = installer::InstallerManager::new(
            "TestApp".to_string(),
            "1.0.0".to_string(),
            "TestPublisher".to_string(),
        );
        let msi_config = installer.create_msi_config();
        assert!(msi_config.is_ok());
        let config = msi_config.unwrap();
        assert_eq!(config.product_name, "TestApp");
        assert_eq!(config.product_version, "1.0.0");
    }

    #[test]
    fn test_msix_config_creation() {
        let installer = installer::InstallerManager::new(
            "TestApp".to_string(),
            "1.0.0".to_string(),
            "TestPublisher".to_string(),
        );
        let msix_config = installer.create_msix_config();
        assert!(msix_config.is_ok());
        let config = msix_config.unwrap();
        assert_eq!(config.package_name, "TestApp");
    }

    #[test]
    fn test_wix_xml_generation() {
        let installer = installer::InstallerManager::new(
            "TestApp".to_string(),
            "1.0.0".to_string(),
            "TestPublisher".to_string(),
        );
        let config = installer.create_msi_config().unwrap();
        let xml = installer.generate_wix_xml(&config);
        assert!(xml.is_ok());
        let xml_content = xml.unwrap();
        assert!(xml_content.contains("TestApp"));
        assert!(xml_content.contains("1.0.0"));
    }

    #[test]
    fn test_appx_manifest_generation() {
        let installer = installer::InstallerManager::new(
            "TestApp".to_string(),
            "1.0.0".to_string(),
            "TestPublisher".to_string(),
        );
        let config = installer.create_msix_config().unwrap();
        let manifest = installer.generate_appx_manifest(&config);
        assert!(manifest.is_ok());
        let manifest_content = manifest.unwrap();
        assert!(manifest_content.contains("TestApp"));
    }

    #[tokio::test]
    async fn test_update_manager() {
        let updater = updater::UpdateManager::new(
            "TestApp".to_string(),
            "1.0.0".to_string(),
            "https://example.com/updates".to_string(),
        );
        let result = updater.check_for_updates().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_notification_manager_creation() {
        let notifications = notifications::NotificationManager::new("TestApp".to_string());
        let toast = notifications::ToastNotification {
            title: "Test".to_string(),
            body: "Test notification".to_string(),
            image_url: None,
            actions: vec![],
            audio: None,
            duration: notifications::NotificationDuration::Short,
            scenario: notifications::NotificationScenario::Default,
        };
        let xml = notifications.generate_toast_xml(&toast);
        assert!(xml.contains("Test"));
        assert!(xml.contains("Test notification"));
    }

    #[test]
    fn test_performance_optimizer() {
        let optimizer = performance::PerformanceOptimizer::new();
        let io_opts = optimizer.optimize_io();
        assert!(io_opts.is_ok());
        let mem_opts = optimizer.optimize_memory();
        assert!(mem_opts.is_ok());
        let net_opts = optimizer.optimize_network();
        assert!(net_opts.is_ok());
    }
}
