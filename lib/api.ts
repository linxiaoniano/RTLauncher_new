import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Account, DeviceCodeInfo, VersionManifest, InstalledVersion, InstallProgressEvent, JavaInstallation, LauncherConfig, ManualJavaEntry } from './types';

// ==================== 账户相关 ====================

// 获取所有账户
export async function getAccounts(): Promise<Account[]> {
  return invoke<Account[]>('get_accounts');
}

// 获取选中的账户
export async function getSelectedAccount(): Promise<Account | null> {
  return invoke<Account | null>('get_selected_account');
}

// 设置选中的账户
export async function selectAccount(accountId: string): Promise<void> {
  return invoke('select_account', { accountId });
}

// 离线登录
export async function loginOffline(username: string): Promise<Account> {
  return invoke<Account>('login_offline', { username });
}

// 开始微软登录 - 获取设备码
export async function startMicrosoftLogin(): Promise<DeviceCodeInfo> {
  return invoke<DeviceCodeInfo>('start_microsoft_login');
}

// 轮询微软登录状态
export async function pollMicrosoftLogin(): Promise<Account> {
  return invoke<Account>('poll_microsoft_login');
}

// 取消微软登录
export async function cancelMicrosoftLogin(): Promise<void> {
  return invoke('cancel_microsoft_login');
}

// 外置登录
export async function loginYggdrasil(
  email: string,
  password: string,
  serverUrl: string
): Promise<Account> {
  return invoke<Account>('login_yggdrasil', { email, password, serverUrl });
}

// 刷新账户令牌
export async function refreshAccount(accountId: string): Promise<Account> {
  return invoke<Account>('refresh_account', { accountId });
}

// 删除账户
export async function removeAccount(accountId: string): Promise<void> {
  return invoke('remove_account', { accountId });
}

// ==================== 版本管理 ====================

// 获取远程版本清单
export async function getVersionManifest(): Promise<VersionManifest> {
  return invoke<VersionManifest>('get_version_manifest');
}

// 获取已安装的版本列表
export async function getInstalledVersions(): Promise<InstalledVersion[]> {
  return invoke<InstalledVersion[]>('get_installed_versions');
}

// 安装版本
export async function installVersion(versionId: string): Promise<InstalledVersion> {
  return invoke<InstalledVersion>('install_version', { versionId });
}

// 监听安装进度
export function onInstallProgress(callback: (progress: InstallProgressEvent) => void) {
  return listen<InstallProgressEvent>('install-progress', (event) => {
    callback(event.payload);
  });
}

// 删除版本
export async function deleteVersion(versionId: string): Promise<void> {
  return invoke('delete_version_cmd', { versionId });
}

// ==================== Java 相关 ====================

// 检测系统 Java 安装
export async function detectJava(): Promise<JavaInstallation[]> {
  return invoke<JavaInstallation[]>('detect_java');
}

// ==================== 启动器配置 ====================

// 获取启动器配置
export async function getLauncherConfig(): Promise<LauncherConfig> {
  return invoke<LauncherConfig>('get_launcher_config');
}

// 设置选中的 Java
export async function setSelectedJava(path: string): Promise<void> {
  return invoke('set_selected_java', { path });
}

// 添加手动 Java
export async function addManualJava(entry: ManualJavaEntry): Promise<void> {
  return invoke('add_manual_java', {
    path: entry.path,
    version: entry.version,
    majorVersion: entry.major_version,
    vendor: entry.vendor,
    arch: entry.arch,
  });
}

// 移除手动 Java
export async function removeManualJava(path: string): Promise<boolean> {
  return invoke<boolean>('remove_manual_java', { path });
}

// 设置内存
export async function setMemory(min: number, max: number): Promise<void> {
  return invoke('set_memory', { min, max });
}

// 设置选中版本
export async function setSelectedVersion(version: string): Promise<void> {
  return invoke('set_selected_version', { version });
}

// ==================== 游戏目录相关 ====================

// 获取游戏目录
export async function getGameDir(): Promise<string> {
  return invoke<string>('get_game_dir_cmd');
}

// 设置游戏目录
export async function setGameDir(path: string): Promise<void> {
  return invoke('set_game_dir', { path });
}

// ==================== 启动相关 ====================

// 启动游戏
export async function launchGame(
  versionId: string,
  javaPath: string,
  javaVersion: number,
  minMemory: number,
  maxMemory: number
): Promise<void> {
  return invoke('launch_game', {
    versionId,
    javaPath,
    javaVersion,
    minMemory,
    maxMemory,
  });
}
