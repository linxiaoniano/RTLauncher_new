"use client"

import * as React from "react"
import { usePathname, useRouter } from "next/navigation"
import {
  Home,
  Download,
  Wrench,
  Settings,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"

interface SidebarProps {
  className?: string
}

interface NavItem {
  icon: React.ReactNode
  label: string
  href: string
  isAvatar?: boolean
}

// 顶部导航项
const topNavItems: NavItem[] = [
  { icon: <Home className="size-4" />, label: "首页", href: "/" },
  { icon: <Download className="size-4" />, label: "下载", href: "/downloads" },
  { icon: <Wrench className="size-4" />, label: "工具", href: "/tools" },
]

// 底部导航项
const bottomNavItems: NavItem[] = [
  { icon: <Settings className="size-4" />, label: "设置", href: "/settings" },
]

// 导航按钮
function NavButton({ item, isActive }: { item: NavItem; isActive: boolean }) {
  const router = useRouter()
  
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button 
          variant={isActive ? "default" : "ghost"} 
          size="icon"
          onClick={() => router.push(item.href)}
        >
          {item.icon}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="right">
        <p>{item.label}</p>
      </TooltipContent>
    </Tooltip>
  )
}

// 左侧边栏
export function Sidebar({ className }: SidebarProps) {
  const pathname = usePathname()
  
  return (
    <aside
      className={cn(
        "flex h-full w-14 flex-col bg-sidebar border-border",
        "glass-sidebar",
        className
      )}
    >
      <nav className="flex flex-1 flex-col items-center gap-2 p-2">
        {topNavItems.map((item, index) => (
          <NavButton 
            key={index} 
            item={item} 
            isActive={pathname === item.href}
          />
        ))}
      </nav>

      <div className="flex flex-col items-center gap-2 border-t border-border p-2">
        {bottomNavItems.map((item, index) => (
          <NavButton 
            key={index} 
            item={item} 
            isActive={pathname === item.href}
          />
        ))}
      </div>
    </aside>
  )
}