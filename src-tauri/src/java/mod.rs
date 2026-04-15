use std::path::PathBuf;
use std::process::Command;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use std::collections::HashSet;

#[derive(Error, Debug)]
pub enum JavaError {
    #[error("未找到Java")]
    NotFound,
    
    #[error("执行Java命令失败: {0}")]
    CommandError(String),
    
    #[error("解析Java版本失败")]
    ParseError,
}

/// Java 安装信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub path: PathBuf,
    pub version: String,
    pub major_version: u32,
    pub vendor: String,
    pub arch: String,
}

impl JavaInstallation {
    /// 获取 java 可执行文件路径
    pub fn java_exec(&self) -> PathBuf {
        if cfg!(windows) {
            self.path.join("bin").join("java.exe")
        } else {
            self.path.join("bin").join("java")
        }
    }
    
    /// 获取 javaw 可执行文件路径 (Windows only)
    pub fn javaw_exec(&self) -> PathBuf {
        if cfg!(windows) {
            self.path.join("bin").join("javaw.exe")
        } else {
            self.java_exec()
        }
    }
}

/// 检测系统中安装的 Java
pub fn detect_java_installations() -> Vec<JavaInstallation> {
    let mut installations = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();
    
    // 从环境变量检测
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let path = PathBuf::from(&java_home);
        if let Some(java) = check_java_installation(&path) {
            seen_paths.insert(path.canonicalize().unwrap_or(path));
            installations.push(java);
        }
    }
    
    // 从 PATH 检测
    if let Some(java) = find_java_in_path() {
        let canonical = java.path.canonicalize().unwrap_or(java.path.clone());
        if seen_paths.insert(canonical) {
            installations.push(java);
        }
    }
    
    // 检测常见安装位置
    for path in get_common_java_paths() {
        if let Some(java) = check_java_installation(&path) {
            let canonical = java.path.canonicalize().unwrap_or(java.path.clone());
            if seen_paths.insert(canonical) {
                installations.push(java);
            }
        }
    }
    
    // 使用系统命令搜索（更快）
    for java in search_java_with_system() {
        let canonical = java.path.canonicalize().unwrap_or(java.path.clone());
        if seen_paths.insert(canonical) {
            installations.push(java);
        }
    }
    
    // 如果还是没找到，尝试更广泛的搜索
    if installations.is_empty() {
        for java in deep_search_java() {
            let canonical = java.path.canonicalize().unwrap_or(java.path.clone());
            if seen_paths.insert(canonical) {
                installations.push(java);
            }
        }
    }
    
    installations
}

/// 在 PATH 中查找 Java
fn find_java_in_path() -> Option<JavaInstallation> {
    let java_name = if cfg!(windows) { "java.exe" } else { "java" };
    
    if let Ok(path) = which::which(java_name) {
        // 获取 java 所在目录的上两级（bin/java -> JAVA_HOME）
        let java_home = path.parent()?.parent()?.to_path_buf();
        
        check_java_installation(&java_home)
    } else {
        None
    }
}

/// 获取常见的 Java 安装路径
fn get_common_java_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        // 获取所有驱动器
        let drives = get_windows_drives();
        
        // 常见的 Java 安装目录名
        let java_dirs = [
            "Java",
            "Eclipse Adoptium",
            "AdoptOpenJDK",
            "Amazon Corretto",
            "Azul Zulu",
            "BellSoft Liberica",
            "Microsoft",
            "OpenJDK",
            "Semeru",
            "GraalVM",
            "jdk",
            "jre",
            "Java\\jdk-*",
            "Java\\jre-*",
        ];
        
        // 遍历所有驱动器的 Program Files 目录
        for drive in &drives {
            let program_files_dirs = [
                drive.join("Program Files"),
                drive.join("Program Files (x86)"),
                drive.join("Program Files (ARM)"),
            ];
            
            for pf in &program_files_dirs {
                if !pf.exists() {
                    continue;
                }
                
                for java_dir in &java_dirs {
                    let java_path = pf.join(java_dir);
                    if java_path.exists() {
                        // 如果是目录，遍历子目录
                        if java_path.is_dir() {
                            if let Ok(entries) = std::fs::read_dir(&java_path) {
                                for entry in entries.flatten() {
                                    let path = entry.path();
                                    // 只添加看起来像 Java 安装的目录
                                    if path.join("bin").exists() {
                                        paths.push(path);
                                    }
                                }
                            }
                        }
                    } else {
                        // 尝试通配符匹配 (jdk-*, jre-*)
                        if java_dir.contains('*') {
                            if let Some(parent) = java_path.parent() {
                                if let Ok(entries) = std::fs::read_dir(parent) {
                                    let prefix = java_dir.replace('*', "");
                                    for entry in entries.flatten() {
                                        let name = entry.file_name();
                                        let name_str = name.to_string_lossy();
                                        if name_str.starts_with(&prefix) && entry.path().join("bin").exists() {
                                            paths.push(entry.path());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 用户目录
        if let Some(home) = dirs::home_dir() {
            // IntelliJ IDEA 等工具安装的 JDK
            let jdks_dir = home.join(".jdks");
            if let Ok(entries) = std::fs::read_dir(&jdks_dir) {
                for entry in entries.flatten() {
                    paths.push(entry.path());
                }
            }
            
            // 用户 AppData
            let appdata_paths = [
                home.join("AppData\\Local\\Programs"),
                home.join("AppData\\Local\\Java"),
                home.join(".sdkman\\candidates\\java"),
            ];
            
            for appdata_path in &appdata_paths {
                if appdata_path.exists() {
                    if let Ok(entries) = std::fs::read_dir(appdata_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.join("bin").exists() {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
            
            // 检查根目录下的 java/jdk 文件夹
            let user_java_dirs = [
                home.join("java"),
                home.join("jdk"),
                home.join("Java"),
                home.join("JDK"),
            ];
            
            for dir in &user_java_dirs {
                if dir.exists() {
                    if let Ok(entries) = std::fs::read_dir(dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.join("bin").exists() {
                                paths.push(path);
                            }
                        }
                    }
                }
            }
        }
        
        // 检查其他常见位置
        let other_paths = [
            "C:\\Windows\\System32", // 有时 Java 会安装在这里
            "C:\\Java",
            "C:\\jdk",
        ];
        
        for path in &other_paths {
            let p = PathBuf::from(path);
            if p.exists() && p.join("bin").exists() {
                paths.push(p);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS
        let library_java = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if let Ok(entries) = std::fs::read_dir(&library_java) {
            for entry in entries.flatten() {
                paths.push(entry.path().join("Contents/Home"));
            }
        }
        
        // 检查用户安装
        if let Some(home) = dirs::home_dir() {
            let user_java = home.join("Library/Java/JavaVirtualMachines");
            if let Ok(entries) = std::fs::read_dir(&user_java) {
                for entry in entries.flatten() {
                    paths.push(entry.path().join("Contents/Home"));
                }
            }
            
            // IntelliJ IDEA 等工具安装的 JDK
            if let Ok(entries) = std::fs::read_dir(home.join(".jdks")) {
                for entry in entries.flatten() {
                    paths.push(entry.path());
                }
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        // Linux 常见路径
        let linux_paths = [
            "/usr/lib/jvm",
            "/usr/lib64/jvm",
            "/usr/lib32/jvm",
            "/usr/java",
            "/usr/jdk",
            "/usr/local/java",
            "/usr/local/jdk",
            "/opt/java",
            "/opt/jdk",
            "/opt/openjdk",
            "/opt/oracle-java",
            "/usr/lib/jvm/default",
            "/usr/lib/jvm/default-runtime",
            "/var/lib/snapd/snap/java", // Snap
            "/app/jdk",                  // Flatpak/AppImage
        ];
        
        for base in &linux_paths {
            let base = PathBuf::from(base);
            if base.exists() {
                if let Ok(entries) = std::fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        paths.push(entry.path());
                    }
                } else {
                    paths.push(base);
                }
            }
        }
        
        // Gentoo/Fedora 等
        for entry in ["java-11-openjdk", "java-17-openjdk", "java-21-openjdk", "java-8-openjdk"] {
            for arch in ["arm64", "aarch64", "x86_64", "amd64", "arm", ""] {
                let path = if arch.is_empty() {
                    format!("/usr/lib/jvm/{}", entry)
                } else {
                    format!("/usr/lib/jvm/{}-{}", entry, arch)
                };
                paths.push(PathBuf::from(path));
            }
        }
        
        // 检查用户安装
        if let Some(home) = dirs::home_dir() {
            // SDKMAN
            let sdkman_java = home.join(".sdkman/candidates/java");
            if let Ok(entries) = std::fs::read_dir(&sdkman_java) {
                for entry in entries.flatten() {
                    paths.push(entry.path());
                }
            }
            
            // IntelliJ IDEA 等工具安装的 JDK
            if let Ok(entries) = std::fs::read_dir(home.join(".jdks")) {
                for entry in entries.flatten() {
                    paths.push(entry.path());
                }
            }
            
            // 用户本地安装
            let local_java = home.join(".local/share/java");
            if let Ok(entries) = std::fs::read_dir(&local_java) {
                for entry in entries.flatten() {
                    paths.push(entry.path());
                }
            }
        }
    }
    
    // Android/Termux 环境
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        let termux_paths = [
            "/data/data/com.termux/files/usr/opt/java",
            "/data/data/com.termux/files/usr/lib/jvm",
            "/data/data/com.termux/files/home/.java",
            "/data/data/com.termux/files/home/jdk",
            "/data/data/com.termux/files/usr/share/java",
            // proot-distro 环境
            "/data/data/com.termux/files/usr/var/lib/proot-distro/installed-rootfs/ubuntu/opt/java",
            "/data/data/com.termux/files/usr/var/lib/proot-distro/installed-rootfs/debian/usr/lib/jvm",
        ];
        
        for path in &termux_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                if let Ok(entries) = std::fs::read_dir(&p) {
                    for entry in entries.flatten() {
                        paths.push(entry.path());
                    }
                } else {
                    paths.push(p);
                }
            }
        }
    }
    
    paths
}

/// 使用系统命令搜索 Java（快速）
fn search_java_with_system() -> Vec<JavaInstallation> {
    let mut results = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        // Windows: 使用 where 命令
        if let Ok(output) = Command::new("cmd")
            .args(["/C", "where java"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let path = PathBuf::from(line.trim());
                    if path.exists() {
                        if let Some(parent) = path.parent() {
                            if let Some(java_home) = parent.parent() {
                                if let Some(java) = check_java_installation(&java_home.to_path_buf()) {
                                    results.push(java);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // 使用 locate 命令（如果有）
        if let Ok(output) = Command::new("locate")
            .args(["-i", "-b", "java"])
            .output()
        {
            if output.status.success() {
                for line in String::from_utf8_lossy(&output.stdout).lines() {
                    let path = PathBuf::from(line);
                    if path.file_name().map(|n| n.to_string_lossy() == "java").unwrap_or(false) {
                        if let Some(parent) = path.parent() {
                            if let Some(java_home) = parent.parent() {
                                if let Some(java) = check_java_installation(&java_home.to_path_buf()) {
                                    results.push(java);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 使用 which -a 查找所有 java
        if let Ok(output) = Command::new("sh")
            .args(["-c", "which -a java 2>/dev/null || which java 2>/dev/null || true"])
            .output()
        {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let path = PathBuf::from(line.trim());
                if path.exists() {
                    if let Some(parent) = path.parent() {
                        if let Some(java_home) = parent.parent() {
                            if let Some(java) = check_java_installation(&java_home.to_path_buf()) {
                                results.push(java);
                            }
                        }
                    }
                }
            }
        }
    }
    
    results
}

/// 深度搜索 Java（较慢但更全面）
fn deep_search_java() -> Vec<JavaInstallation> {
    let mut results = Vec::new();
    
    // 定义要搜索的根目录
    let search_roots: Vec<PathBuf> = if cfg!(target_os = "linux") {
        vec![
            PathBuf::from("/usr"),
            PathBuf::from("/opt"),
            PathBuf::from("/home"),
            PathBuf::from("/data"),
        ]
    } else if cfg!(target_os = "windows") {
        // Windows: 获取所有驱动器
        get_windows_drives()
    } else if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/Library"),
        ]
    } else {
        return results;
    };
    
    // 使用 walkdir 递归搜索
    for root in search_roots {
        if !root.exists() {
            continue;
        }
        
        use walkdir::WalkDir;
        
        for entry in WalkDir::new(&root)
            .follow_links(true)
            .max_depth(8) // Windows 可能路径较深
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            // 检查是否是 java 可执行文件
            if name == "java" || name == "java.exe" {
                if let Some(bin_dir) = path.parent() {
                    if bin_dir.file_name().map(|n| n.to_string_lossy() == "bin").unwrap_or(false) {
                        if let Some(java_home) = bin_dir.parent() {
                            if let Some(java) = check_java_installation(&java_home.to_path_buf()) {
                                results.push(java);
                            }
                        }
                    }
                }
            }
        }
    }
    
    results
}

/// 获取 Windows 所有驱动器
#[cfg(target_os = "windows")]
fn get_windows_drives() -> Vec<PathBuf> {
    let mut drives = Vec::new();
    
    // 常见的驱动器字母
    for letter in b'A'..=b'Z' {
        let drive = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive);
        if path.exists() {
            drives.push(path);
        }
    }
    
    drives
}

#[cfg(not(target_os = "windows"))]
fn get_windows_drives() -> Vec<PathBuf> {
    Vec::new()
}

/// 检查指定路径是否是有效的 Java 安装
pub fn check_java_installation(path: &PathBuf) -> Option<JavaInstallation> {
    let java_exec = if cfg!(windows) {
        path.join("bin").join("java.exe")
    } else {
        path.join("bin").join("java")
    };
    
    if !java_exec.exists() {
        return None;
    }
    
    // 执行 java -version 获取版本信息
    #[cfg(target_os = "windows")]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new(&java_exec)
            .arg("-version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?
    };
    
    #[cfg(not(target_os = "windows"))]
    let output = Command::new(&java_exec)
        .arg("-version")
        .output()
        .ok()?;
    
    // java -version 输出到 stderr
    let version_output = String::from_utf8_lossy(&output.stderr);
    
    parse_java_version(&version_output, path)
}

/// 解析 Java 版本输出
fn parse_java_version(output: &str, path: &PathBuf) -> Option<JavaInstallation> {
    // 提取版本号
    // 格式1: java version "1.8.0_292"
    // 格式2: java version "11.0.11"
    // 格式3: openjdk version "17.0.1"
    
    let version_re = Regex::new(r#"version \"?(\d+)(?:\.(\d+)(?:\.(\d+)(?:_(\d+))?)?)?\"?"#).ok()?;
    let vendor_re = Regex::new(r"(OpenJDK|Java|HotSpot|AdoptOpenJDK|Temurin|Corretin|Azul Zulu|Alpine|Alibaba|Liberica)").ok()?;
    
    let version_caps = version_re.captures(output)?;
    let vendor_caps = vendor_re.captures(output);
    
    let major: u32 = version_caps.get(1)?.as_str().parse().ok()?;
    
    let (major_version, version) = if major == 1 {
        // 旧版本格式 (1.8.x)
        let minor: u32 = version_caps.get(2)
            .and_then(|m| m.as_str().parse().ok())
            .unwrap_or(8);
        let patch = version_caps.get(3).map(|m| m.as_str()).unwrap_or("0");
        let build = version_caps.get(4).map(|m| m.as_str()).unwrap_or("");
        
        let version = if build.is_empty() {
            format!("1.{}.{}", minor, patch)
        } else {
            format!("1.{}.{}_{}", minor, patch, build)
        };
        
        (minor, version)
    } else {
        // 新版本格式 (17.x.x)
        let minor = version_caps.get(2).map(|m| m.as_str()).unwrap_or("0");
        let patch = version_caps.get(3).map(|m| m.as_str()).unwrap_or("0");
        
        (major, format!("{}.{}.{}", major, minor, patch))
    };
    
    let vendor = vendor_caps
        .map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string());
    
    // 检测架构
    let arch_re = Regex::new(r"(\d+)-?bit").ok()?;
    let arch = arch_re.captures(output)
        .map(|c| c.get(1).map(|m| m.as_str()).unwrap_or("64"))
        .unwrap_or("64");
    
    Some(JavaInstallation {
        path: path.clone(),
        version,
        major_version,
        vendor,
        arch: format!("{}-bit", arch),
    })
}

/// 验证 Java 是否满足版本要求
pub fn check_java_version(java: &JavaInstallation, required: u32) -> bool {
    java.major_version >= required
}

/// 获取推荐的 Java 版本
pub fn get_recommended_java_version(mc_version: u32) -> u32 {
    match mc_version {
        0..=16 => 8,   // 1.16 及以下推荐 Java 8
        17..=17 => 17, // 1.17 需要 Java 16+
        _ => 17,       // 1.18+ 需要 Java 17+
    }
}
