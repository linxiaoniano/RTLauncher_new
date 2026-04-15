"use client"

import * as React from "react"
import { Download, Search, Loader2, Check, Filter } from "lucide-react"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Progress } from "@/components/ui/progress"
import { 
  getVersionManifest, 
  installVersion, 
  getInstalledVersions,
  onInstallProgress 
} from "@/lib/api"
import type { ManifestVersion, InstalledVersion, InstallProgressEvent, VersionType } from "@/lib/types"

const VERSION_TYPE_LABELS: Record<VersionType, { label: string; color: string; icon: string }> = {
  release: { label: '正式版', color: 'bg-green-500', icon: '🎮' },
  snapshot: { label: '快照', color: 'bg-yellow-500', icon: '🔧' },
  old_alpha: { label: '远古版 (Alpha)', color: 'bg-purple-500', icon: '👾' },
  old_beta: { label: '远古版 (Beta)', color: 'bg-purple-500', icon: '🎲' },
}

// 特殊版本标签
const SPECIAL_VERSIONS: Record<string, string> = {
  '2.0': '愚人节版',
  '15w14a': '愚人节版',
  '3D Shareware v1.34': '愚人节版',
  '23w13a_or_b': '愚人节版',
  '24w14potato': '愚人节版',
}

export default function DownloadsPage() {
  const [manifest, setManifest] = React.useState<ManifestVersion[]>([])
  const [installed, setInstalled] = React.useState<Set<string>>(new Set())
  const [loading, setLoading] = React.useState(true)
  const [search, setSearch] = React.useState('')
  const [filter, setFilter] = React.useState<VersionType | 'all' | 'april_fools'>('all')
  const [sortBy, setSortBy] = React.useState<'newest' | 'oldest' | 'version'>('newest')
  const [installing, setInstalling] = React.useState<string | null>(null)
  const [progress, setProgress] = React.useState<InstallProgressEvent | null>(null)

  React.useEffect(() => {
    loadData()
    
    const unlisten = onInstallProgress((p) => {
      setProgress(p)
    })
    
    return () => {
      unlisten.then(fn => fn())
    }
  }, [])

  const loadData = async () => {
    try {
      const [manifestData, installedData] = await Promise.all([
        getVersionManifest(),
        getInstalledVersions()
      ])
      setManifest(manifestData.versions)
      setInstalled(new Set(installedData.map(v => v.id)))
    } catch (e) {
      console.error('Failed to load data:', e)
    } finally {
      setLoading(false)
    }
  }

  const handleInstall = async (versionId: string) => {
    setInstalling(versionId)
    setProgress({ stage: 'downloading_json', current: 0, total: 1, message: '开始安装...' })
    
    try {
      await installVersion(versionId)
      setInstalled(prev => new Set([...prev, versionId]))
    } catch (e) {
      console.error('Install failed:', e)
      alert(e instanceof Error ? e.message : '安装失败')
    } finally {
      setInstalling(null)
      setProgress(null)
    }
  }

  const filteredVersions = React.useMemo(() => {
    let result = manifest
    
    if (filter === 'april_fools') {
      // 筛选愚人节版本
      result = result.filter(v => SPECIAL_VERSIONS[v.id] !== undefined)
    } else if (filter !== 'all') {
      result = result.filter(v => v.type === filter)
    }
    
    if (search) {
      const query = search.toLowerCase()
      result = result.filter(v => v.id.toLowerCase().includes(query))
    }
    
    // 排序
    if (sortBy === 'newest') {
      result = [...result].sort((a, b) => new Date(b.releaseTime).getTime() - new Date(a.releaseTime).getTime())
    } else if (sortBy === 'oldest') {
      result = [...result].sort((a, b) => new Date(a.releaseTime).getTime() - new Date(b.releaseTime).getTime())
    } else if (sortBy === 'version') {
      result = [...result].sort((a, b) => b.id.localeCompare(a.id, undefined, { numeric: true }))
    }
    
    return result
  }, [manifest, filter, search, sortBy])

  if (loading) {
    return (
      <div className="flex items-center justify-center h-96">
        <Loader2 className="size-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">下载游戏</h1>
          <p className="text-muted-foreground">下载 Minecraft 游戏版本</p>
        </div>
      </div>

      {/* 安装进度 */}
      {installing && progress && (
        <Card className="border-primary">
          <CardContent className="pt-4">
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span>{progress.message}</span>
                <span>{Math.round(progress.current / progress.total * 100)}%</span>
              </div>
              <Progress value={progress.current / progress.total * 100} />
            </div>
          </CardContent>
        </Card>
      )}

      {/* 搜索和筛选 */}
      <div className="space-y-4">
        <div className="flex gap-4">
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 size-4 text-muted-foreground" />
            <Input
              placeholder="搜索版本，例如：1.20.4、1.7.10、2.0..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-10"
            />
          </div>
          <div className="flex items-center gap-2">
            <Filter className="size-4 text-muted-foreground" />
            <select
              value={sortBy}
              onChange={(e) => setSortBy(e.target.value as 'newest' | 'oldest' | 'version')}
              className="text-sm border rounded-md px-2 py-1 bg-background"
            >
              <option value="newest">最新优先</option>
              <option value="oldest">最早优先</option>
              <option value="version">版本号</option>
            </select>
          </div>
        </div>
        
        <div className="flex gap-2 flex-wrap">
          <Button
            variant={filter === 'all' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('all')}
          >
            全部
          </Button>
          <Button
            variant={filter === 'release' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('release')}
          >
            正式版
          </Button>
          <Button
            variant={filter === 'snapshot' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('snapshot')}
          >
            快照
          </Button>
          <Button
            variant={filter === 'old_alpha' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('old_alpha')}
          >
            远古版
          </Button>
          <Button
            variant={filter === 'april_fools' ? 'default' : 'outline'}
            size="sm"
            onClick={() => setFilter('april_fools')}
          >
            🎭 愚人节版
          </Button>
        </div>
      </div>

      {/* 版本列表 */}
      <div className="space-y-2">
        {filteredVersions.slice(0, 100).map((version) => {
          const isInstalled = installed.has(version.id)
          const isInstalling = installing === version.id
          const typeInfo = VERSION_TYPE_LABELS[version.type]
          const isAprilFools = SPECIAL_VERSIONS[version.id] !== undefined
          
          return (
            <Card key={version.id} className="p-4">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <div className={`w-2 h-2 rounded-full ${typeInfo.color}`} />
                  <div>
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-medium">{version.id}</span>
                      <Badge variant="secondary" className="text-xs">
                        {typeInfo.label}
                      </Badge>
                      {isAprilFools && (
                        <Badge className="bg-pink-500 text-white text-xs">
                          🎭 愚人节版
                        </Badge>
                      )}
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {new Date(version.releaseTime).toLocaleDateString('zh-CN', { 
                        year: 'numeric', 
                        month: 'long', 
                        day: 'numeric' 
                      })}
                    </p>
                  </div>
                </div>
                <Button
                  size="sm"
                  variant={isInstalled ? 'outline' : 'default'}
                  disabled={isInstalling || isInstalled}
                  onClick={() => handleInstall(version.id)}
                >
                  {isInstalling ? (
                    <Loader2 className="size-4 animate-spin mr-2" />
                  ) : isInstalled ? (
                    <Check className="size-4 mr-2" />
                  ) : (
                    <Download className="size-4 mr-2" />
                  )}
                  {isInstalling ? '安装中' : isInstalled ? '已安装' : '安装'}
                </Button>
              </div>
            </Card>
          )
        })}
      </div>

      {filteredVersions.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          没有找到匹配的版本
        </div>
      )}
    </div>
  )
}
