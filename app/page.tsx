"use client"

import * as React from "react"
import { Play, Download, Settings, User, Loader2, Plus, Trash2, RefreshCw, Check, AlertCircle } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { useRouter } from "next/navigation"
import { AccountPanel } from "@/components/account-panel"
import { 
  getInstalledVersions, 
  getVersionManifest, 
  installVersion, 
  deleteVersion,
  detectJava,
  launchGame,
  onInstallProgress,
  getSelectedAccount,
  getLauncherConfig,
  setSelectedVersion as saveSelectedVersion,
} from "@/lib/api"
import type { InstalledVersion, ManifestVersion, JavaInstallation, InstallProgressEvent } from "@/lib/types"

// Java 选择器组件
function JavaSelector({
  selectedJava,
  javaInstallations,
  scanning,
  onSelect,
  onSettings
}: {
  selectedJava: JavaInstallation | null
  javaInstallations: JavaInstallation[]
  scanning?: boolean
  onSelect: (java: JavaInstallation) => void
  onSettings: () => void
}) {
  // 显示最多2个Java选项
  const displayItems = javaInstallations.slice(0, 2)
  
  return (
    <>
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium">
          选择 Java
          {scanning && <Loader2 className="inline-block ml-2 size-3 animate-spin" />}
        </span>
        <Button variant="ghost" size="sm" onClick={onSettings}>
          <Settings className="size-4" />
        </Button>
      </div>
      {selectedJava ? (
        <div className="flex items-center gap-3 p-2 rounded-lg border bg-muted/50">
          <div className="flex-1">
            <p className="font-medium">Java {selectedJava.major_version}</p>
            <p className="text-xs text-muted-foreground truncate">{selectedJava.vendor}</p>
          </div>
          <Check className="size-4 text-primary" />
        </div>
      ) : displayItems.length > 0 ? (
        <div className="space-y-2">
          {displayItems.map((java) => (
            <div
              key={java.path}
              className="flex items-center gap-3 p-2 rounded-lg border cursor-pointer hover:bg-muted/50"
              onClick={() => onSelect(java)}
            >
              <div className="flex-1">
                <p className="font-medium">Java {java.major_version}</p>
                <p className="text-xs text-muted-foreground">{java.vendor}</p>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <p className="text-sm text-muted-foreground">
          未检测到 Java，请手动添加
        </p>
      )}
    </>
  )
}

export default function Page() {
  const router = useRouter()
  const [installedVersions, setInstalledVersions] = React.useState<InstalledVersion[]>([])
  const [selectedVersion, setSelectedVersion] = React.useState<string | null>(null)
  const [javaInstallations, setJavaInstallations] = React.useState<JavaInstallation[]>([])
  const [selectedJava, setSelectedJava] = React.useState<JavaInstallation | null>(null)
  const [loading, setLoading] = React.useState(true)
  const [javaScanning, setJavaScanning] = React.useState(false)
  const [installing, setInstalling] = React.useState<string | null>(null)
  const [installProgress, setInstallProgress] = React.useState<InstallProgressEvent | null>(null)
  const [launching, setLaunching] = React.useState(false)
  const [hasAccount, setHasAccount] = React.useState(false)
  const [minMemory, setMinMemory] = React.useState(512)
  const [maxMemory, setMaxMemory] = React.useState(4096)

  // 加载数据
  React.useEffect(() => {
    loadData()
    
    // 监听安装进度
    const unlisten = onInstallProgress((progress) => {
      setInstallProgress(progress)
    })
    
    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  const loadData = async () => {
    try {
      // 先快速加载版本和账户信息
      const [versions, account] = await Promise.all([
        getInstalledVersions(),
        getSelectedAccount()
      ])
      setInstalledVersions(versions)
      setHasAccount(!!account)
      
      // 从后端加载配置
      try {
        const config = await getLauncherConfig()
        if (config.min_memory > 0) setMinMemory(config.min_memory)
        if (config.max_memory > 0) setMaxMemory(config.max_memory)
        
        // 加载保存的 Java 设置（先使用缓存的）
        if (config.selected_java_path) {
          setSelectedJava({
            path: config.selected_java_path,
            version: 'Java',
            major_version: 17,
            vendor: '已保存',
            arch: '64-bit'
          })
        }
        
        // 加载保存的版本选择
        if (config.selected_version) {
          // 如果保存的版本在已安装列表中，选中它
          const found = versions.find(v => v.id === config.selected_version)
          if (found) {
            setSelectedVersion(found.id)
          }
        }
      } catch (e) {
        console.error('Failed to load launcher config:', e)
      }
      
      // 显示界面，不再等待 Java 扫描
      setLoading(false)
      
      // 自动选择第一个版本（如果没有保存的选择）
      if (versions.length > 0 && !selectedVersion) {
        setSelectedVersion(versions[0].id)
      }
      
      // 后台扫描 Java
      setJavaScanning(true)
      detectJava().then(javas => {
        setJavaInstallations(javas)
        
        // 如果有保存的 Java，尝试匹配
        const savedJavaPath = localStorage.getItem('selectedJava')
        if (savedJavaPath) {
          const found = javas.find(j => j.path === savedJavaPath)
          if (found) {
            setSelectedJava(found)
          }
        } else if (javas.length > 0 && !selectedJava) {
          setSelectedJava(javas[0])
          localStorage.setItem('selectedJava', javas[0].path)
        }
      }).catch(console.error).finally(() => {
        setJavaScanning(false)
      })
      
    } catch (e) {
      console.error('Failed to load data:', e)
      setLoading(false)
    }
  }

  // 启动游戏
  const handleLaunch = async () => {
    if (!selectedVersion || !selectedJava) return
    
    setLaunching(true)
    try {
      await launchGame(
        selectedVersion,
        selectedJava.path,
        selectedJava.major_version,
        minMemory,
        maxMemory
      )
    } catch (e) {
      console.error('Launch failed:', e)
      alert(e instanceof Error ? e.message : String(e))
    } finally {
      setLaunching(false)
    }
  }

  // 安装最新版本
  const handleInstallLatest = async () => {
    try {
      const manifest = await getVersionManifest()
      const latestRelease = manifest.versions.find(v => v.id === manifest.latest.release)
      if (latestRelease) {
        setInstalling(latestRelease.id)
        setInstallProgress({ stage: 'downloading_json', current: 0, total: 1, message: '开始安装...' })
        
        await installVersion(latestRelease.id)
        
        // 刷新列表
        const versions = await getInstalledVersions()
        setInstalledVersions(versions)
        if (!selectedVersion && versions.length > 0) {
          setSelectedVersion(versions[0].id)
        }
      }
    } catch (e) {
      console.error('Install failed:', e)
      alert(e instanceof Error ? e.message : String(e))
    } finally {
      setInstalling(null)
      setInstallProgress(null)
    }
  }

  // 删除版本
  const handleDeleteVersion = async (versionId: string) => {
    if (!confirm(`确定要删除版本 ${versionId} 吗？`)) return
    
    try {
      await deleteVersion(versionId)
      setInstalledVersions(prev => prev.filter(v => v.id !== versionId))
      if (selectedVersion === versionId) {
        setSelectedVersion(installedVersions[0]?.id || null)
      }
    } catch (e) {
      console.error('Delete failed:', e)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <Loader2 className="size-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      {/* 欢迎区域 */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">RTLauncher</h1>
          <p className="text-muted-foreground">Minecraft 游戏启动器</p>
        </div>
        <Button 
          size="lg" 
          onClick={handleLaunch}
          disabled={!selectedVersion || !selectedJava || launching || !hasAccount}
        >
          {launching ? (
            <Loader2 className="size-4 mr-2 animate-spin" />
          ) : (
            <Play className="size-4 mr-2" />
          )}
          {launching ? '启动中...' : '启动游戏'}
        </Button>
      </div>

      {/* 警告提示 */}
      {!selectedJava && (
        <Alert variant="destructive">
          <AlertCircle className="size-4" />
          <AlertDescription>
            未检测到 Java，请在 <Button variant="link" className="p-0 h-auto" onClick={() => router.push('/settings')}>设置</Button> 中手动添加
          </AlertDescription>
        </Alert>
      )}
      
      {!hasAccount && (
        <Alert>
          <AlertCircle className="size-4" />
          <AlertDescription>
            请先登录账户才能启动游戏
          </AlertDescription>
        </Alert>
      )}

      {/* 安装进度 */}
      {installing && installProgress && (
        <Card className="border-primary">
          <CardContent className="pt-4">
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>{installProgress.message}</span>
                <span>{Math.round(installProgress.current / installProgress.total * 100)}%</span>
              </div>
              <Progress value={installProgress.current / installProgress.total * 100} />
            </div>
          </CardContent>
        </Card>
      )}

      <div className="grid gap-6 lg:grid-cols-3">
        {/* 游戏版本 */}
        <Card className="lg:col-span-2">
          <CardHeader>
            <div className="flex items-center justify-between">
              <div>
                <CardTitle className="flex items-center gap-2">
                  <Download className="size-5" />
                  游戏版本
                </CardTitle>
                <CardDescription>
                  选择要启动的游戏版本
                </CardDescription>
              </div>
              <div className="flex gap-2">
                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={() => router.push('/downloads')}
                >
                  <Download className="size-4 mr-2" />
                  更多版本
                </Button>
                <Button 
                  variant="outline" 
                  size="sm"
                  onClick={handleInstallLatest}
                  disabled={!!installing}
                >
                  {installing ? (
                    <Loader2 className="size-4 mr-2 animate-spin" />
                  ) : (
                    <Plus className="size-4 mr-2" />
                  )}
                  安装最新版
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {installedVersions.length === 0 ? (
              <div className="text-center py-8">
                <Download className="size-12 mx-auto text-muted-foreground mb-3" />
                <p className="text-muted-foreground mb-4">还没有安装任何游戏版本</p>
                <div className="flex gap-2 justify-center">
                  <Button onClick={() => router.push('/downloads')}>
                    <Download className="size-4 mr-2" />
                    浏览版本
                  </Button>
                  <Button variant="outline" onClick={handleInstallLatest} disabled={!!installing}>
                    <Plus className="size-4 mr-2" />
                    安装最新版
                  </Button>
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                {installedVersions.map(version => (
                  <div
                    key={version.id}
                    className={`flex items-center gap-3 p-3 rounded-lg border cursor-pointer transition-all ${
                      selectedVersion === version.id
                        ? 'ring-2 ring-primary bg-primary/5'
                        : 'hover:bg-muted/50'
                    }`}
                    onClick={() => setSelectedVersion(version.id)}
                  >
                    <div className="flex-1">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{version.id}</span>
                        <Badge variant="secondary">
                          {version.version_type}
                        </Badge>
                      </div>
                      <p className="text-xs text-muted-foreground">
                        Java {version.java_version}+ · {version.libraries_count} 个库文件
                      </p>
                    </div>
                    {selectedVersion === version.id && (
                      <Check className="size-4 text-primary" />
                    )}
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation()
                        handleDeleteVersion(version.id)
                      }}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* 右侧面板 */}
        <div className="space-y-6">
          {/* Java 设置 */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="flex items-center gap-2">
                  <Settings className="size-5" />
                  Java
                </CardTitle>
                <Button variant="ghost" size="sm" onClick={() => router.push('/settings')}>
                  设置
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              <JavaSelector 
                selectedJava={selectedJava} 
                javaInstallations={javaInstallations}
                scanning={javaScanning}
                onSelect={setSelectedJava} 
                onSettings={() => router.push('/settings')}
              />
            </CardContent>
          </Card>

          {/* 账户管理 */}
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <User className="size-5" />
                账户
              </CardTitle>
              <CardDescription>
                登录后才能启动游戏
              </CardDescription>
            </CardHeader>
            <CardContent>
              <AccountPanel onAccountChange={() => setHasAccount(true)} />
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  )
}
