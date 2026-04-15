"use client"

import * as React from "react"
import { User, Crown, Server, Trash2, RefreshCw, Check } from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import type { Account } from "@/lib/types"

interface AccountCardProps {
  account: Account
  isSelected?: boolean
  onSelect?: () => void
  onRemove?: () => void
  onRefresh?: () => void
  refreshing?: boolean
}

export function AccountCard({ 
  account, 
  isSelected, 
  onSelect, 
  onRemove, 
  onRefresh,
  refreshing 
}: AccountCardProps) {
  const typeConfig = {
    offline: { label: '离线', icon: User, color: 'bg-gray-500' },
    microsoft: { label: '微软', icon: Crown, color: 'bg-blue-500' },
    yggdrasil: { label: '外置', icon: Server, color: 'bg-green-500' },
  }
  
  const config = typeConfig[account.account_type]
  const TypeIcon = config.icon

  return (
    <div 
      className={cn(
        "group relative flex items-center gap-3 p-3 rounded-xl border transition-all cursor-pointer",
        "hover:bg-muted/50",
        isSelected && "ring-2 ring-primary bg-primary/5"
      )}
      onClick={onSelect}
    >
      {/* 选中指示器 */}
      {isSelected && (
        <div className="absolute -left-1 top-1/2 -translate-y-1/2 w-1 h-8 bg-primary rounded-full" />
      )}
      
      {/* 头像 */}
      <Avatar className="size-12">
        <AvatarImage src={account.skin_url} alt={account.username} />
        <AvatarFallback className="text-lg font-bold">
          {account.username.charAt(0).toUpperCase()}
        </AvatarFallback>
      </Avatar>
      
      {/* 信息 */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-medium truncate">{account.username}</span>
          <Badge variant="secondary" className="text-xs">
            <TypeIcon className="size-3 mr-1" />
            {config.label}
          </Badge>
        </div>
        <p className="text-xs text-muted-foreground truncate">
          {account.uuid.slice(0, 8)}...{account.uuid.slice(-4)}
        </p>
      </div>
      
      {/* 操作菜单 */}
      <DropdownMenu>
        <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
          <Button variant="ghost" size="icon" className="opacity-0 group-hover:opacity-100">
            <svg className="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="1" />
              <circle cx="12" cy="5" r="1" />
              <circle cx="12" cy="19" r="1" />
            </svg>
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          {account.account_type !== 'offline' && (
            <>
              <DropdownMenuItem onClick={onRefresh} disabled={refreshing}>
                <RefreshCw className={cn("size-4 mr-2", refreshing && "animate-spin")} />
                刷新令牌
              </DropdownMenuItem>
              <DropdownMenuSeparator />
            </>
          )}
          <DropdownMenuItem 
            onClick={onRemove}
            className="text-destructive focus:text-destructive"
          >
            <Trash2 className="size-4 mr-2" />
            删除账户
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      
      {/* 选中标记 */}
      {isSelected && (
        <div className="absolute right-3 top-3">
          <Check className="size-4 text-primary" />
        </div>
      )}
    </div>
  )
}
