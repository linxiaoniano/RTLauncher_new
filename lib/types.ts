// 账户类型
export type AccountType = 'offline' | 'microsoft' | 'yggdrasil';

// 账户信息
export interface Account {
  id: string;
  account_type: AccountType;
  username: string;
  uuid: string;
  skin_url?: string;
  cape_url?: string;
  access_token?: string;
  refresh_token?: string;
  expires_at?: number;
  auth_server_url?: string;
}

// 设备码信息（微软登录用）
export interface DeviceCodeInfo {
  verification_uri: string;
  user_code: string;
  expires_in: number;
  interval: number;
}

// 版本类型
export type VersionType = 'release' | 'snapshot' | 'old_alpha' | 'old_beta';

// 版本清单中的版本信息
export interface ManifestVersion {
  id: string;
  type: VersionType;
  url: string;
  time: string;
  releaseTime: string;
  sha1?: string;
}

// 版本清单
export interface VersionManifest {
  latest: {
    release: string;
    snapshot: string;
  };
  versions: ManifestVersion[];
}

// 已安装的版本信息
export interface InstalledVersion {
  id: string;
  version_type: VersionType;
  main_class: string;
  java_version: number;
  assets_id: string;
  libraries_count: number;
  installed_at: number;
}

// 安装进度事件
export interface InstallProgressEvent {
  stage: 'downloading_json' | 'downloading_client' | 'downloading_libraries' | 'downloading_assets' | 'extracting_natives' | 'completed';
  current: number;
  total: number;
  message: string;
}

// Java 安装信息
export interface JavaInstallation {
  path: string;
  version: string;
  major_version: number;
  vendor: string;
  arch: string;
}

// 手动添加的 Java 条目
export interface ManualJavaEntry {
  path: string;
  version: string;
  major_version: number;
  vendor: string;
  arch: string;
}

// 启动器配置
export interface LauncherConfig {
  selected_java_path: string | null;
  manual_java_paths: ManualJavaEntry[];
  min_memory: number;
  max_memory: number;
  game_dir: string | null;
  selected_version: string | null;
}
