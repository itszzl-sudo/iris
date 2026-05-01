use iris_jetcrab_engine::NpmDownloader;
use std::path::PathBuf;

#[test]
fn test_internal_package_detection() {
    // 测试内部包识别
    assert!(NpmDownloader::is_internal_package("iris"));
    assert!(NpmDownloader::is_internal_package("@irisverse/iris"));
    assert!(NpmDownloader::is_internal_package("iris-runtime"));
    assert!(NpmDownloader::is_internal_package("iris-core"));
    assert!(NpmDownloader::is_internal_package("iris-gpu"));
    assert!(NpmDownloader::is_internal_package("iris-layout"));
    assert!(NpmDownloader::is_internal_package("iris-dom"));
    assert!(NpmDownloader::is_internal_package("iris-sfc"));
    assert!(NpmDownloader::is_internal_package("iris-cssom"));
    assert!(NpmDownloader::is_internal_package("iris-jetcrab"));
    assert!(NpmDownloader::is_internal_package("iris-jetcrab-engine"));
    assert!(NpmDownloader::is_internal_package("iris-jetcrab-cli"));

    // 测试外部包（应该返回 false）
    assert!(!NpmDownloader::is_internal_package("vue"));
    assert!(!NpmDownloader::is_internal_package("pinia"));
    assert!(!NpmDownloader::is_internal_package("@vue/runtime-core"));
    assert!(!NpmDownloader::is_internal_package("axios"));
    assert!(!NpmDownloader::is_internal_package("lodash"));
}

#[test]
fn test_download_internal_package_should_fail() {
    // 测试下载内部包应该失败
    let temp_dir = std::env::temp_dir().join("test_internal_pkg");
    let downloader = NpmDownloader::new(temp_dir.clone());

    // 尝试下载内部包
    let result = downloader.download_and_install("iris", None);
    assert!(result.is_err(), "Downloading internal package should fail");
    
    let result = downloader.download_and_install("iris-runtime", None);
    assert!(result.is_err(), "Downloading iris-runtime should fail");

    // 清理
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_package_json_filtering() {
    // 这个测试验证 package.json 解析时会过滤内部包
    // 实际测试在 vue_compiler 的集成测试中进行
    
    // 模拟 package.json 内容
    let package_json = r#"{
        "name": "test-project",
        "version": "1.0.0",
        "dependencies": {
            "vue": "^3.5.0",
            "iris": "^0.1.0",
            "iris-runtime": "^0.1.0",
            "pinia": "^2.1.0"
        },
        "devDependencies": {
            "iris-core": "^0.1.0",
            "typescript": "^5.0.0"
        }
    }"#;

    // 解析并验证
    let json: serde_json::Value = serde_json::from_str(package_json).unwrap();
    let mut external_count = 0;
    
    if let Some(deps) = json.get("dependencies").and_then(|v| v.as_object()) {
        for (name, _) in deps {
            if !NpmDownloader::is_internal_package(name) {
                external_count += 1;
                println!("External dependency: {}", name);
            } else {
                println!("Filtered internal package: {}", name);
            }
        }
    }
    
    if let Some(deps) = json.get("devDependencies").and_then(|v| v.as_object()) {
        for (name, _) in deps {
            if !NpmDownloader::is_internal_package(name) {
                external_count += 1;
                println!("External devDependency: {}", name);
            } else {
                println!("Filtered internal devDependency: {}", name);
            }
        }
    }

    // 应该只有 vue, pinia, typescript 三个外部包
    assert_eq!(external_count, 3, "Should have 3 external packages");
}

#[test]
#[ignore] // 需要网络连接
fn test_external_package_download() {
    // 测试外部包可以正常下载
    let temp_dir = std::env::temp_dir().join("test_external_pkg");
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    let downloader = NpmDownloader::new(temp_dir.clone());
    
    // 下载外部包应该成功
    let result = downloader.download_and_install("vue", Some("3.5.33"));
    assert!(result.is_ok(), "External package download should succeed");
    
    let path = result.unwrap();
    assert!(path.exists());
    assert!(path.join("package.json").exists());
    
    // 清理
    let _ = std::fs::remove_dir_all(&temp_dir);
}
