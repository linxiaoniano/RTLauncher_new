"use client"

import * as React from "react"
import { User, Plus, Loader2 } from "lucide-react"
import { Button } from "@/components/ui/button"
import { AccountCard } from "@/components/account-card"
import { LoginDialog } from "@/components/login-dialog"
import { 
  getAccounts, 
  getSelectedAccount, 
  selectAccount, 
  removeAccount, 
  refreshAccount 
} from "@/lib/api"
import type { Account } from "@/lib/types"

interface AccountPanelProps {
  onAccountChange?: () => void
}

export function AccountPanel({ onAccountChange }: AccountPanelProps) {
  const [accounts, setAccounts] = React.useState<Account[]>([])
  const [selectedId, setSelectedId] = React.useState<string | null>(null)
  const [loading, setLoading] = React.useState(true)
  const [refreshingId, setRefreshingId] = React.useState<string | null>(null)

  // 加载账户列表
  const loadAccounts = React.useCallback(async () => {
    try {
      const [accountList, selected] = await Promise.all([
        getAccounts(),
        getSelectedAccount()
      ])
      setAccounts(accountList)
      setSelectedId(selected?.id ?? null)
    } catch (e) {
      console.error('Failed to load accounts:', e)
    } finally {
      setLoading(false)
    }
  }, [])

  React.useEffect(() => {
    loadAccounts()
  }, [loadAccounts])

  // 选择账户
  const handleSelect = async (accountId: string) => {
    try {
      await selectAccount(accountId)
      setSelectedId(accountId)
      onAccountChange?.()
    } catch (e) {
      console.error('Failed to select account:', e)
    }
  }

  // 删除账户
  const handleRemove = async (accountId: string) => {
    try {
      await removeAccount(accountId)
      setAccounts(prev => prev.filter(a => a.id !== accountId))
      if (selectedId === accountId) {
        setSelectedId(null)
      }
      onAccountChange?.()
    } catch (e) {
      console.error('Failed to remove account:', e)
    }
  }

  // 刷新令牌
  const handleRefresh = async (accountId: string) => {
    setRefreshingId(accountId)
    try {
      const updated = await refreshAccount(accountId)
      setAccounts(prev => prev.map(a => a.id === accountId ? updated : a))
    } catch (e) {
      console.error('Failed to refresh account:', e)
    } finally {
      setRefreshingId(null)
    }
  }

  // 登录成功回调
  const handleLoginSuccess = () => {
    loadAccounts()
    onAccountChange?.()
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="size-6 animate-spin text-muted-foreground" />
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* 账户列表 */}
      {accounts.length === 0 ? (
        <div className="text-center py-8">
          <User className="size-12 mx-auto text-muted-foreground mb-3" />
          <p className="text-muted-foreground mb-4">还没有添加任何账户</p>
          <LoginDialog onLoginSuccess={handleLoginSuccess}>
            <Button>
              <Plus className="size-4 mr-2" />
              添加账户
            </Button>
          </LoginDialog>
        </div>
      ) : (
        <>
          <div className="space-y-2">
            {accounts.map(account => (
              <AccountCard
                key={account.id}
                account={account}
                isSelected={account.id === selectedId}
                onSelect={() => handleSelect(account.id)}
                onRemove={() => handleRemove(account.id)}
                onRefresh={() => handleRefresh(account.id)}
                refreshing={refreshingId === account.id}
              />
            ))}
          </div>
          
          <LoginDialog 
            onLoginSuccess={handleLoginSuccess}
            trigger={
              <Button variant="outline" className="w-full">
                <Plus className="size-4 mr-2" />
                添加账户
              </Button>
            }
          />
        </>
      )}
    </div>
  )
}
