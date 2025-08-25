"use client";
import Link from "next/link"
import { useEffect, useRef, useState } from "react";
import DepositDialog from "./DepositDialog";
import WithdrawDialog from "./WithdrawDialog";
import { MdOutlineLogout } from "react-icons/md";
import {
    Menubar,
    MenubarContent,
    MenubarItem,
    MenubarMenu,
    MenubarSeparator,
    MenubarTrigger,
  } from "@/components/ui/menubar";
  import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { CgProfile } from "react-icons/cg";
import { useAuth } from "@/store/auth";
import { useRouter } from "next/navigation";
import { IoClose, IoSearch } from "react-icons/io5";
import Wallet from "./Wallet";
import { Gi3dGlasses } from "react-icons/gi";
import SearchDialog from "./SearchDialog";
  
const Navbar = () => {
    const [q, setQ] = useState("");
    const [isSearchFocused, setIsSearchFocused] = useState(false);
    const [depositOpen, setDepositOpen] = useState(false);
    const [withdrawOpen, setWithdrawOpen] = useState(false);
    const { logout } = useAuth();
    const router = useRouter();
    const searchRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
        if (searchRef.current && !searchRef.current.contains(event.target as Node)) {
            setIsSearchFocused(false);
        }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
}, []);
  const handleMarketClick = (symbol: string, id: string) => {
    router.push(`/trade/${symbol}?id=${id}`);
    setQ("");
    setIsSearchFocused(false);
};

  const clearSearch = () => {
    setQ("");
    setIsSearchFocused(false);
};
  return (
    <header className="sticky top-0 z-20 border-b border-white/10 bg-black/80 backdrop-blur">
    <div className="mx-auto flex justify-between h-16 max-w-7xl items-center gap-3 px-4">
      {/* Left: Logo + name */}
      <Link href="/" className="flex items-center gap-2">
          <div className="flex items-center gap-3 font-semibold">
            <Gi3dGlasses size={24}/>
            <span className="text-xl font-bold">Trueman</span>
          </div>
      </Link>
      {/* Middle: Search + Spot pill */}
      <div className="mx-3 flex items-center gap-3 relative">
        <div className="hidden items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 md:flex md:min-w-[340px]">
          <IoSearch />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            onFocus={() => setIsSearchFocused(true)}
            placeholder="Search markets (e.g. BTC, SOL)"
            className="w-full bg-transparent text-sm outline-none placeholder:text-zinc-400"
          />
          {q && (
              <button
                  onClick={clearSearch}
                  className="text-zinc-400 hover:text-white transition-colors"
              >
                  <IoClose size={16} />
              </button>
          )}
        </div>
        {isSearchFocused && (
          <SearchDialog
            searchRef={searchRef as React.RefObject<HTMLDivElement>}
            handleMarketClick={handleMarketClick}
            query={q}
          />
        )}
        <span className="hidden rounded-full border border-emerald-500/20 bg-emerald-500/10 px-2.5 py-1 text-xs font-semibold text-emerald-300 md:inline">
          Spot
        </span>
      </div>

      {/* Right: Actions */}
      <div className="flex items-center gap-2">
        <button
          onClick={() => setDepositOpen(true)}
          className="md:flex justify-center items-center hidden h-9 rounded-lg border border-white/15 bg-white/5 px-3 text-sm font-medium hover:bg-white/10"
        >
          Deposit
        </button>
        <button
          onClick={() => setWithdrawOpen(true)}
          className="md:flex justify-center items-center hidden h-9 rounded-lg bg-gradient-to-r from-violet-500 to-cyan-400 px-3 text-sm font-semibold text-black hover:brightness-110"
        >
          Withdraw
        </button>
        {/* Avatar (placeholder) */}
        <Menubar className="bg-transparent border-none">
            <MenubarMenu>
                <MenubarTrigger className="bg-transparent px-0 cursor-pointer">
                    <Avatar>
                        <AvatarImage src="https://github.com/evilrabbit.png" />
                        <AvatarFallback>CN</AvatarFallback>
                    </Avatar>
                </MenubarTrigger>
                <MenubarContent>
                <MenubarItem className="flex items-center gap-4">
                    <div className="flex items-center gap-2 h-full">
                        <CgProfile size={50} />
                    </div>
                    <div className="flex flex-col items-start">
                        Ashutosh Sagar
                        <span className="text-xs text-muted-foreground">
                            ashutoshsagar@gmail.com
                        </span>
                    </div>
                </MenubarItem>
                <MenubarSeparator />
                <MenubarItem>
                    <Wallet />
                </MenubarItem>
                <MenubarSeparator />
                <MenubarItem 
                    className="flex items-center gap-2 cursor-pointer"
                    onClick={() => {
                        logout();
                        router.push("/");
                    }}
                ><MdOutlineLogout size={20} />Logout</MenubarItem>
                </MenubarContent>
            </MenubarMenu>
        </Menubar>
      </div>
    </div>
    <DepositDialog open={depositOpen} onOpenChange={setDepositOpen} />
    <WithdrawDialog open={withdrawOpen} onOpenChange={setWithdrawOpen} />
  </header>

  )
}

export default Navbar