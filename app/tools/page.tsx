"use client"

import * as React from "react"
import { Wrench, FolderOpen, FileText, AlertCircle, FolderPlus } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { open } from "@tauri-apps/plugin-dialog"
import { invoke } from "@tauri-apps/api/core"
import { getGameDir, setGameDir } from "@/lib/api"

export default function ToolsPage() {
  const [gameDir, setGameDirState] = React.useState<string | null>(null)
  const [dirInfo, setDirInfo] = React.useState<{
    versionsCount: number
    modsCount: number
  } | null>(null)

  // 从后端加载游戏目录
  React.useEffect(() => {
    loadGameDir()
  }, [])

  const loadGameDir = async () => {
    try {
      const dir = await getGameDir()
      setGameDirState(dir)
      
      // 尝试从已安装的版本获取信息
      import('@/lib/api').then(({ getInstalledVersions }) => {
        getInstalledVersions().then(versions => {
          setDirInfo({
            versionsCount: versions.length,
            modsCount: 0 // 需要后端支持才能获取
          })
        })
      })
    } catch (e) {
      console.error('Failed to load game dir:', e)
    }
  }

  // 选择游戏目录
  const handleSelectGameDir = async () => {
    try {
      const selected = await open({
        title: '选择游戏目录',
        directory: true
      })
      
      if (selected && typeof selected === 'string') {
        // 同步到后端
        await setGameDir(selected)
        setGameDirState(selected)
        localStorage.setItem('gameDir', selected)
      }
    } catch (e) {
      console.error('Failed to select directory:', e)
    }
  }

  // 打开目录的通用函数
  const openDirectory = async (path: string, fallbackMessage: string) => {
    try {
      await invoke('open_path', { path })
    } catch {
      try {
        // 尝试 alternative 命令
        await invoke('open_directory', { path })
      } catch {
        // 如果后端不支持，显示路径信息
        alert(fallbackMessage + '\n\n' + path)
      }
    }
  }

  return (
    <div className="p-6 space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold">工具</h1>
        <p className="text-muted-foreground">游戏目录管理和实用工具</p>
      </div>

      {/* 游戏目录管理 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FolderOpen className="size-5" />
            游戏目录
          </CardTitle>
          <CardDescription>
            设置和管理 Minecraft 游戏目录
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {!gameDir ? (
            <Alert>
              <AlertCircle className="size-4" />
              <AlertDescription>
                尚未设置游戏目录，点击下方按钮选择你的 .minecraft 文件夹
              </AlertDescription>
            </Alert>
          ) : (
            <div className="space-y-4">
              <div className="p-3 rounded-lg border bg-muted/50">
                <p className="text-sm font-medium mb-1">当前游戏目录</p>
                <p className="text-xs text-muted-foreground font-mono break-all">{gameDir}</p>
              </div>

              {dirInfo && (
                <div className="grid grid-cols-2 gap-3">
                  <div className="p-3 rounded-lg border">
                    <p className="text-sm font-medium">已安装版本</p>
                    <p className="text-2xl font-bold">{dirInfo.versionsCount}</p>
                  </div>
                  <div className="p-3 rounded-lg border">
                    <p className="text-sm font-medium">Mods</p>
                    <p className="text-2xl font-bold">{dirInfo.modsCount}</p>
                    <p className="text-xs text-muted-foreground">需要后端支持</p>
                  </div>
                </div>
              )}

              <div className="flex gap-2">
                <Button 
                  onClick={() => openDirectory(gameDir, '请手动打开以下目录：')} 
                  className="flex-1"
                >
                  <FolderOpen className="size-4 mr-2" />
                  打开目录
                </Button>
                <Button variant="outline" onClick={handleSelectGameDir}>
                  <FolderPlus className="size-4 mr-2" />
                  更换目录
                </Button>
              </div>
            </div>
          )}

          <Button onClick={handleSelectGameDir} className="w-full">
            <FolderOpen className="size-4 mr-2" />
            {gameDir ? '更换游戏目录' : '选择游戏目录'}
          </Button>
        </CardContent>
      </Card>

      {/* 实用工具 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wrench className="size-5" />
            快捷操作
          </CardTitle>
          <CardDescription>
            快速访问游戏相关文件
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <div className="flex items-center justify-between p-3 rounded-lg border">
            <div>
              <p className="font-medium">打开存档目录</p>
              <p className="text-sm text-muted-foreground">快速访问游戏存档文件夹</p>
            </div>
            <Button 
              variant="outline" 
              disabled={!gameDir}
              onClick={() => {
                if (gameDir) {
                  const savesPath = gameDir + '/saves'
                  openDirectory(savesPath, '存档目录位置：')
                }
              }}
            >
              <FileText className="size-4 mr-2" />
              打开
            </Button>
          </div>

          <div className="flex items-center justify-between p-3 rounded-lg border">
            <div>
              <p className="font-medium">打开资源包目录</p>
              <p className="text-sm text-muted-foreground">管理你的资源包和材质包</p>
            </div>
            <Button 
              variant="outline" 
              disabled={!gameDir}
              onClick={() => {
                if (gameDir) {
                  const rpPath = gameDir + '/resourcepacks'
                  openDirectory(rpPath, '资源包目录位置：')
                }
              }}
            >
              <FileText className="size-4 mr-2" />
              打开
            </Button>
          </div>

          <div className="flex items-center justify-between p-3 rounded-lg border">
            <div>
              <p className="font-medium">打开截图目录</p>
              <p className="text-sm text-muted-foreground">查看你拍摄的游戏截图</p>
            </div>
            <Button 
              variant="outline" 
              disabled={!gameDir}
              onClick={() => {
                if (gameDir) {
                  const screenshotsPath = gameDir + '/screenshots'
                  openDirectory(screenshotsPath, '截图目录位置：')
                }
              }}
            >
              <FileText className="size-4 mr-2" />
              打开
            </Button>
          </div>

          <div className="flex items-center justify-between p-3 rounded-lg border">
            <div>
              <p className="font-medium">打开 Mods 目录</p>
              <p className="text-sm text-muted-foreground">管理你的 Mods 文件</p>
            </div>
            <Button 
              variant="outline" 
              disabled={!gameDir}
              onClick={() => {
                if (gameDir) {
                  const modsPath = gameDir + '/mods'
                  openDirectory(modsPath, 'Mods 目录位置：')
                }
              }}
            >
              <FileText className="size-4 mr-2" />
              打开
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* 关于启动器 */}
      <Card>
        <CardHeader>
          <CardTitle>关于 RTLauncher</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">版本</span>
              <Badge>v0.1.0</Badge>
            </div>
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">构建类型</span>
              <Badge variant="secondary">开发版</Badge>
            </div>
            <p className="text-sm text-muted-foreground mt-2">
              一个使用 Tauri + Next.js 构建的现代化 Minecraft 启动器
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
