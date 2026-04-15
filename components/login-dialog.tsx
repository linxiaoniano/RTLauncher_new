"use client"

import * as React from "react"
import { User, Monitor, Loader2, ExternalLink, Copy, Check } from "lucide-react"
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field"
import { loginOffline, startMicrosoftLogin, pollMicrosoftLogin, cancelMicrosoftLogin, loginYggdrasil } from "@/lib/api"
import type { Account, DeviceCodeInfo } from "@/lib/types"
import { openUrl } from "@tauri-apps/plugin-opener"

type LoginType = 'offline' | 'microsoft' | 'yggdrasil'

interface LoginDialogProps {
  children?: React.ReactNode
  onLoginSuccess?: (account: Account) => void
  trigger?: React.ReactNode
}

export function LoginDialog({ onLoginSuccess, trigger }: LoginDialogProps) {
  const [open, setOpen] = React.useState(false)
  const [loginType, setLoginType] = React.useState<LoginType>('offline')
  const [loading, setLoading] = React.useState(false)
  const [error, setError] = React.useState<string | null>(null)
  
  // 离线登录
  const [offlineUsername, setOfflineUsername] = React.useState('')
  
  // 微软登录
  const [deviceCode, setDeviceCode] = React.useState<DeviceCodeInfo | null>(null)
  const [copied, setCopied] = React.useState(false)
  
  // 外置登录
  const [ygEmail, setYgEmail] = React.useState('')
  const [ygPassword, setYgPassword] = React.useState('')
  const [ygServerUrl, setYgServerUrl] = React.useState('https://littleskin.cn/api/yggdrasil')

  // 重置状态
  React.useEffect(() => {
    if (!open) {
      setLoginType('offline')
      setLoading(false)
      setError(null)
      setOfflineUsername('')
      setDeviceCode(null)
      setCopied(false)
      setYgEmail('')
      setYgPassword('')
    }
  }, [open])

  // 处理离线登录
  const handleOfflineLogin = async () => {
    if (!offlineUsername.trim()) {
      setError('请输入用户名')
      return
    }
    
    setLoading(true)
    setError(null)
    
    try {
      const account = await loginOffline(offlineUsername.trim())
      setOpen(false)
      onLoginSuccess?.(account)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }

  // 开始微软登录
  const handleStartMicrosoftLogin = async () => {
    setLoading(true)
    setError(null)
    
    try {
      const code = await startMicrosoftLogin()
      setDeviceCode(code)
      // 尝试自动打开浏览器
      try {
        await openUrl(code.verification_uri)
      } catch {
        // 如果无法自动打开浏览器，用户可以手动复制链接
        console.log('无法自动打开浏览器，用户需要手动复制链接')
      }
      startPolling()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      setLoading(false)
    }
  }

  // 轮询微软登录状态
  const startPolling = () => {
    const poll = async () => {
      try {
        const account = await pollMicrosoftLogin()
        setOpen(false)
        setDeviceCode(null)
        setLoading(false)
        onLoginSuccess?.(account)
      } catch (e) {
        const message = e instanceof Error ? e.message : String(e)
        if (message.includes('等待用户授权')) {
          // 继续轮询
          setTimeout(poll, 3000)
        } else {
          setError(message || '登录失败')
          setLoading(false)
          setDeviceCode(null)
        }
      }
    }
    poll()
  }

  // 取消微软登录
  const handleCancelMicrosoftLogin = async () => {
    try {
      await cancelMicrosoftLogin()
    } catch {}
    setDeviceCode(null)
    setLoading(false)
  }

  // 复制设备码
  const copyUserCode = async () => {
    if (deviceCode) {
      try {
        await navigator.clipboard.writeText(deviceCode.user_code)
        setCopied(true)
        setTimeout(() => setCopied(false), 2000)
      } catch {}
    }
  }

  // 打开验证链接
  const openVerificationUrl = async () => {
    if (deviceCode) {
      try {
        await openUrl(deviceCode.verification_uri)
      } catch {}
    }
  }

  // 处理外置登录
  const handleYggdrasilLogin = async () => {
    if (!ygEmail.trim()) {
      setError('请输入邮箱')
      return
    }
    if (!ygPassword.trim()) {
      setError('请输入密码')
      return
    }
    if (!ygServerUrl.trim()) {
      setError('请输入服务器地址')
      return
    }
    
    setLoading(true)
    setError(null)
    
    try {
      const account = await loginYggdrasil(ygEmail.trim(), ygPassword, ygServerUrl.trim())
      setOpen(false)
      onLoginSuccess?.(account)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }

  // 处理登录按钮点击
  const handleLoginClick = () => {
    if (loginType === 'offline') {
      handleOfflineLogin()
    } else if (loginType === 'yggdrasil') {
      handleYggdrasilLogin()
    }
  }

  return (
    <AlertDialog open={open} onOpenChange={setOpen}>
      <AlertDialogTrigger asChild>
        {trigger || <Button><User className="size-4 mr-2" />添加账户</Button>}
      </AlertDialogTrigger>
      <AlertDialogContent className="max-w-md">
        <AlertDialogHeader>
          <AlertDialogMedia>
            <User className="size-8" />
          </AlertDialogMedia>
          <AlertDialogTitle>添加账户</AlertDialogTitle>
          <AlertDialogDescription>
            选择登录方式来添加你的 Minecraft 账户
          </AlertDialogDescription>
        </AlertDialogHeader>

        {/* 登录类型选择 */}
        {!deviceCode && (
          <div className="flex gap-2 mb-4">
            <Button
              type="button"
              variant={loginType === 'offline' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setLoginType('offline')}
              className="flex-1"
            >
              离线
            </Button>
            <Button
              type="button"
              variant={loginType === 'microsoft' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setLoginType('microsoft')}
              className="flex-1"
            >
              微软
            </Button>
            <Button
              type="button"
              variant={loginType === 'yggdrasil' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setLoginType('yggdrasil')}
              className="flex-1"
            >
              外置
            </Button>
          </div>
        )}

        {/* 错误提示 */}
        {error && (
          <div className="bg-destructive/10 text-destructive text-sm p-3 rounded-lg">
            {error}
          </div>
        )}

        {/* 离线登录表单 */}
        {loginType === 'offline' && !deviceCode && (
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="offline-username">用户名</FieldLabel>
              <Input
                id="offline-username"
                placeholder="输入游戏内显示的用户名"
                value={offlineUsername}
                onChange={(e) => setOfflineUsername(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleOfflineLogin()}
              />
            </Field>
          </FieldGroup>
        )}

        {/* 微软登录 */}
        {loginType === 'microsoft' && !deviceCode && (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              点击下方按钮开始微软登录，将自动打开浏览器进行授权。
            </p>
            <Button 
              type="button"
              onClick={handleStartMicrosoftLogin} 
              disabled={loading}
              className="w-full"
            >
              {loading ? <Loader2 className="size-4 animate-spin mr-2" /> : <Monitor className="size-4 mr-2" />}
              开始微软登录
            </Button>
          </div>
        )}

        {/* 微软登录 - 设备码显示 */}
        {deviceCode && (
          <div className="space-y-4">
            <div className="bg-muted p-4 rounded-lg text-center space-y-3">
              <p className="text-sm text-muted-foreground">访问以下网址并输入验证码</p>
              <div className="bg-background p-3 rounded-md flex items-center justify-between gap-2">
                <span className="text-sm font-mono break-all flex-1 text-left">
                  {deviceCode.verification_uri}
                </span>
                <Button 
                  type="button" 
                  variant="ghost" 
                  size="sm"
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(deviceCode.verification_uri)
                    } catch {}
                  }}
                >
                  <Copy className="size-3" />
                </Button>
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={openVerificationUrl}
                className="w-full"
              >
                <ExternalLink className="size-3 mr-2" />
                在浏览器中打开
              </Button>
            </div>
            
            <div className="bg-muted p-4 rounded-lg text-center space-y-3">
              <p className="text-sm text-muted-foreground">验证码</p>
              <div className="flex items-center justify-center gap-2">
                <button
                  type="button"
                  onClick={copyUserCode}
                  className="text-3xl font-mono font-bold tracking-widest bg-background px-4 py-2 rounded-md hover:bg-muted transition-colors cursor-pointer"
                >
                  {deviceCode.user_code}
                </button>
                <Button type="button" variant="ghost" size="icon" onClick={copyUserCode}>
                  {copied ? <Check className="size-4 text-green-500" /> : <Copy className="size-4" />}
                </Button>
              </div>
              <p className="text-xs text-muted-foreground">
                {copied ? '已复制！' : '点击验证码即可复制'}
              </p>
            </div>

            <div className="bg-yellow-500/10 border border-yellow-500/20 p-3 rounded-lg">
              <p className="text-xs text-yellow-600 dark:text-yellow-400">
                💡 提示：如果无法自动打开浏览器，请手动点击上方链接并输入验证码
              </p>
            </div>
            
            <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="size-4 animate-spin" />
              等待授权中...
            </div>
            <Button type="button" variant="outline" onClick={handleCancelMicrosoftLogin} className="w-full">
              取消
            </Button>
          </div>
        )}

        {/* 外置登录表单 */}
        {loginType === 'yggdrasil' && !deviceCode && (
          <FieldGroup>
            <Field>
              <FieldLabel htmlFor="yg-email">邮箱</FieldLabel>
              <Input
                id="yg-email"
                type="email"
                placeholder="输入注册邮箱"
                value={ygEmail}
                onChange={(e) => setYgEmail(e.target.value)}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="yg-password">密码</FieldLabel>
              <Input
                id="yg-password"
                type="password"
                placeholder="输入密码"
                value={ygPassword}
                onChange={(e) => setYgPassword(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleYggdrasilLogin()}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="yg-server">验证服务器</FieldLabel>
              <Input
                id="yg-server"
                placeholder="https://littleskin.cn/api/yggdrasil"
                value={ygServerUrl}
                onChange={(e) => setYgServerUrl(e.target.value)}
              />
            </Field>
          </FieldGroup>
        )}

        {/* 底部按钮 */}
        {!deviceCode && (
          <AlertDialogFooter>
            <AlertDialogCancel>取消</AlertDialogCancel>
            {loginType !== 'microsoft' && (
              <Button 
                type="button"
                onClick={handleLoginClick}
                disabled={loading}
              >
                {loading && <Loader2 className="size-4 animate-spin mr-2" />}
                登录
              </Button>
            )}
          </AlertDialogFooter>
        )}
      </AlertDialogContent>
    </AlertDialog>
  )
}