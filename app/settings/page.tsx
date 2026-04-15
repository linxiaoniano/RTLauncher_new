"use client"

import * as React from "react"
import { Settings, FolderOpen, Plus, Trash2, Check, AlertCircle } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { detectJava, getLauncherConfig, addManualJava, removeManualJava, setSelectedJava, setMemory } from "@/lib/api"
import type { JavaInstallation, ManualJavaEntry } from "@/lib/types"
import { open } from "@tauri-apps/plugin-dialog"

export default function SettingsPage() {
  const [javaInstallations, setJavaInstallations] = React.useState<JavaInstallation[]>([])
  const [selectedJava, setSelectedJavaState] = React.useState<string | null>(null)
  const [loading, setLoading] = React.useState(true)
  const [javaScanning, setJavaScanning] = React.useState(false)
  const [manualPath, setManualPath] = React.useState('')
  const [manualVersion, setManualVersion] = React.useState(17)
  const [minMemory, setMinMemoryState] = React.useState(512)
  const [maxMemory, setMaxMemoryState] = React.useState(4096)

  // 加载设置
  React.useEffect(() => {
    loadSettings()
  }, [])

  const loadSettings = async () => {
    try {
      // 从后端加载配置
      const config = await getLauncherConfig()
      
      if (config.selected_java_path) {
        setSelectedJavaState(config.selected_java_path)
      }
      if (config.min_memory > 0) {
        setMinMemoryState(config.min_memory)
      }
      if (config.max_memory > 0) {
        setMaxMemoryState(config.max_memory)
      }
      
      // 后台扫描 Java
      setJavaScanning(true)
      const javas = await detectJava()
      
      // 合并后端保存的手动 Java 条目
      const manualEntries: JavaInstallation[] = config.manual_java_paths.map((m: ManualJavaEntry) => ({
        path: m.path,
        version: m.version,
        major_version: m.major_version,
        vendor: m.vendor,
        arch: m.arch,
      }))
      
      // 合并自动检测到的和手动添加的（去重）
      const seenPaths = new Set(javas.map(j => j.path))
      const uniqueManual = manualEntries.filter(m => !seenPaths.has(m.path))
      const allJava = [...javas, ...uniqueManual]
      
      setJavaInstallations(allJava)
      
      // 如果没有保存的设置，自动选择第一个
      if (!config.selected_java_path && allJava.length > 0) {
        setSelectedJavaState(allJava[0].path)
      }
    } catch (e) {
      console.error('Failed to load settings:', e)
    } finally {
      setLoading(false)
      setJavaScanning(false)
    }
  }

  const handleSelectJava = async (path: string) => {
    setSelectedJavaState(path)
    try {
      await setSelectedJava(path)
    } catch (e) {
      console.error('Failed to save Java selection:', e)
    }
  }

  const handleBrowseJava = async () => {
    try {
      const selected = await open({
        title: '选择 Java 可执行文件',
        filters: [{
          name: 'Java',
          extensions: ['exe', '*']
        }]
      })
      
      if (selected && typeof selected === 'string') {
        // 转换为 JAVA_HOME 格式（去掉 bin/java 部分）
        const javaHome = selected.replace(/[/\\]bin[/\\]java(\.exe)?$/i, '')
        
        const newJava: JavaInstallation = {
          path: javaHome,
          version: `Java ${manualVersion}`,
          major_version: manualVersion,
          vendor: '手动添加',
          arch: '64-bit'
        }
        
        // 保存到后端
        try {
          await addManualJava({
            path: newJava.path,
            version: newJava.version,
            major_version: newJava.major_version,
            vendor: newJava.vendor,
            arch: newJava.arch,
          })
        } catch (e) {
          console.error('Failed to save manual Java:', e)
        }
        
        setJavaInstallations(prev => {
          const filtered = prev.filter(j => j.path !== newJava.path)
          return [...filtered, newJava]
        })
        handleSelectJava(newJava.path)
        setManualPath('')
      }
    } catch (e) {
      console.error('Failed to browse:', e)
    }
  }

  const handleRemoveJava = async (path: string) => {
    setJavaInstallations(prev => prev.filter(j => j.path !== path))
    if (selectedJava === path) {
      setSelectedJavaState(null)
    }
    try {
      await removeManualJava(path)
    } catch (e) {
      console.error('Failed to remove manual Java:', e)
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <div className="animate-spin size-8 border-2 border-primary border-t-transparent rounded-full" />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold">设置</h1>
        <p className="text-muted-foreground">配置启动器选项</p>
      </div>

      {/* Java 设置 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Settings className="size-5" />
            Java 设置
          </CardTitle>
          <CardDescription>
            选择用于启动游戏的 Java 版本
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {javaInstallations.length === 0 && (
            <Alert>
              <AlertCircle className="size-4" />
              <AlertDescription>
                未检测到 Java 安装，请手动添加
              </AlertDescription>
            </Alert>
          )}

          <div className="space-y-2">
            {javaInstallations.map((java) => (
              <div
                key={java.path}
                className={`flex items-center justify-between p-3 rounded-lg border cursor-pointer transition-all ${
                  selectedJava === java.path
                    ? 'ring-2 ring-primary bg-primary/5'
                    : 'hover:bg-muted/50'
                }`}
                onClick={() => handleSelectJava(java.path)}
              >
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <span className="font-medium">Java {java.major_version}</span>
                      <Badge variant="secondary" className="text-xs">{java.vendor}</Badge>
                    </div>
                    <p className="text-xs text-muted-foreground truncate max-w-md">
                      {java.path}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  {selectedJava === java.path && (
                    <Check className="size-4 text-primary" />
                  )}
                  {java.vendor === '手动添加' && (
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={(e) => {
                        e.stopPropagation()
                        handleRemoveJava(java.path)
                      }}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  )}
                </div>
              </div>
            ))}
          </div>

          {/* 手动添加 Java */}
          <div className="border-t pt-4 mt-4">
            <p className="text-sm font-medium mb-3">手动添加 Java</p>
            <div className="space-y-3">
              <div>
                <label className="text-xs text-muted-foreground mb-1 block">
                  Java 路径（完整路径到 java 可执行文件）
                </label>
                <div className="flex gap-2">
                  <Input
                    placeholder="/path/to/java 或 C:\Program Files\Java\jdk-17\bin\java.exe"
                    value={manualPath}
                    onChange={(e) => setManualPath(e.target.value)}
                    className="flex-1"
                  />
                  <Button variant="outline" onClick={handleBrowseJava}>
                    <FolderOpen className="size-4" />
                  </Button>
                </div>
              </div>
              
              <div className="flex gap-2 items-center">
                <div className="flex-1">
                  <label className="text-xs text-muted-foreground mb-1 block">
                    Java 版本号
                  </label>
                  <Input
                    type="number"
                    placeholder="17"
                    value={manualVersion}
                    onChange={(e) => setManualVersion(parseInt(e.target.value) || 17)}
                    className="w-24"
                  />
                </div>
                <Button 
                  className="flex-1 mt-5"
                  onClick={async () => {
                    if (!manualPath.trim()) {
                      alert('请输入 Java 路径')
                      return
                    }
                    const newJava: JavaInstallation = {
                      path: manualPath.trim(),
                      version: `Java ${manualVersion}`,
                      major_version: manualVersion,
                      vendor: '手动添加',
                      arch: '64-bit'
                    }
                    // 保存到后端
                    try {
                      await addManualJava({
                        path: newJava.path,
                        version: newJava.version,
                        major_version: newJava.major_version,
                        vendor: newJava.vendor,
                        arch: newJava.arch,
                      })
                    } catch (e) {
                      console.error('Failed to save manual Java:', e)
                    }
                    setJavaInstallations(prev => {
                      const filtered = prev.filter(j => j.path !== newJava.path)
                      return [...filtered, newJava]
                    })
                    handleSelectJava(newJava.path)
                    setManualPath('')
                  }}
                >
                  <Plus className="size-4 mr-2" />
                  添加 Java
                </Button>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 游戏设置 */}
      <Card>
        <CardHeader>
          <CardTitle>游戏设置</CardTitle>
          <CardDescription>
            内存分配和其他游戏选项
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="text-sm font-medium">最小内存 (MB)</label>
              <Input
                type="number"
                value={minMemory}
                onChange={async (e) => {
                  const val = parseInt(e.target.value) || 512
                  setMinMemoryState(val)
                  try {
                    await setMemory(val, maxMemory)
                  } catch (e) {
                    console.error('Failed to save memory:', e)
                  }
                }}
              />
            </div>
            <div>
              <label className="text-sm font-medium">最大内存 (MB)</label>
              <Input
                type="number"
                value={maxMemory}
                onChange={async (e) => {
                  const val = parseInt(e.target.value) || 4096
                  setMaxMemoryState(val)
                  try {
                    await setMemory(minMemory, val)
                  } catch (e) {
                    console.error('Failed to save memory:', e)
                  }
                }}
              />
            </div>
          </div>
        </CardContent>
      </Card>

      {/* 关于 */}
      <Card>
        <CardHeader>
          <CardTitle>关于</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            RTLauncher v0.1.0<br />
            一个现代化的 Minecraft 启动器
          </p>
        </CardContent>
      </Card>
    </div>
  )
}
