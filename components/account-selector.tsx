"use client"

import * as React from "react"
import { User, ChevronDown, Crown, Server, Plus } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { LoginDialog } from "@/components/login-dialog"
import { getAccounts, getSelectedAccount, selectAccount } from "@/lib/api"
import type { Account } from "@/lib/types"

interface AccountSelectorProps {
  onAccountChange?: () => void
}

export function AccountSelector({ onAccountChange }: AccountSelectorProps) {
  const [accounts, setAccounts] = React.useState<Account[]>([])
  const [selected, setSelected] = React.useState<Account | null>(null)
  const [loading, setLoading] = React.useState(true)

  const loadAccounts = React.useCallback(async () => {
    try {
      const [accountList, selectedAccount] = await Promise.all([
        getAccounts(),
        getSelectedAccount()
      ])
      setAccounts(accountList)
      setSelected(selectedAccount)
    } catch (e) {
      console.error('Failed to load accounts:', e)
    } finally {
      setLoading(false)
    }
  }, [])

  React.useEffect(() => {
    loadAccounts()
  }, [loadAccounts])

  const handleSelect = async (accountId: string) => {
    try {
      await selectAccount(accountId)
      const account = accounts.find(a => a.id === accountId)
      setSelected(account ?? null)
      onAccountChange?.()
    } catch (e) {
      console.error('Failed to select account:', e)
    }
  }

  const handleLoginSuccess = () => {
    loadAccounts()
    onAccountChange?.()
  }

  if (loading) {
    return (
      <Button variant="ghost" size="sm" disabled>
        <User className="size-4 mr-2" />
        加载中...
      </Button>
    )
  }

  if (!selected) {
    return (
      <LoginDialog onLoginSuccess={handleLoginSuccess}>
        <Button variant="ghost" size="sm">
          <User className="size-4 mr-2" />
          未登录
        </Button>
      </LoginDialog>
    )
  }

  const typeConfig = {
    offline: { label: '离线', icon: User },
    microsoft: { label: '微软', icon: Crown },
    yggdrasil: { label: '外置', icon: Server },
  }
  
  const config = typeConfig[selected.account_type]

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="sm" className="gap-2">
          <Avatar className="size-6">
            <AvatarImage src={selected.skin_url} />
            <AvatarFallback className="text-xs">
              {selected.username.charAt(0).toUpperCase()}
            </AvatarFallback>
          </Avatar>
          <span className="hidden sm:inline max-w-24 truncate">
            {selected.username}
          </span>
          <ChevronDown className="size-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel className="flex items-center gap-2">
          <config.icon className="size-4" />
          {config.label}账户
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        
        {accounts.map(account => (
          <DropdownMenuItem
            key={account.id}
            onClick={() => handleSelect(account.id)}
            className="flex items-center gap-2"
          >
            <Avatar className="size-6">
              <AvatarImage src={account.skin_url} />
              <AvatarFallback className="text-xs">
                {account.username.charAt(0).toUpperCase()}
              </AvatarFallback>
            </Avatar>
            <span className="flex-1 truncate">{account.username}</span>
            {account.id === selected.id && (
              <Badge variant="secondary" className="text-xs">当前</Badge>
            )}
          </DropdownMenuItem>
        ))}
        
        {accounts.length > 0 && <DropdownMenuSeparator />}
        
        <LoginDialog onLoginSuccess={handleLoginSuccess}>
          <DropdownMenuItem onSelect={(e) => e.preventDefault()}>
            <Plus className="size-4 mr-2" />
            添加账户
          </DropdownMenuItem>
        </LoginDialog>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
