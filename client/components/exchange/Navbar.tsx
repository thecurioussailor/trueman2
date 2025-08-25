"use client";
import Link from "next/link"
import { useMemo, useState } from "react";
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
import jim from "@/public/jim-carrey.svg";
import { IoSearch } from "react-icons/io5";
import Wallet from "./Wallet";
import { Gi3dGlasses } from "react-icons/gi";
type Market = {
    symbol: string;    // e.g. "SOL/USDC"
    price: string;     // e.g. "168.24"
    change24h: string; // "+2.1%" or "-0.9%"
    volume24h: string; // e.g. "2.3M SOL"
  };
  
  const MOCK_MARKETS: Market[] = [
    { symbol: "BTC/USDC", price: "68420.12", change24h: "+2.4%", volume24h: "12,345 BTC" },
    { symbol: "ETH/USDC", price: "3420.08", change24h: "-1.2%", volume24h: "98,210 ETH" },
    { symbol: "SOL/USDC", price: "168.02", change24h: "+5.1%", volume24h: "2,340,100 SOL" },
  ];
  
const Navbar = () => {
    const [q, setQ] = useState("");
    const [depositOpen, setDepositOpen] = useState(false);
    const [withdrawOpen, setWithdrawOpen] = useState(false);
    const { logout } = useAuth();
    const router = useRouter();
  const filtered = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return MOCK_MARKETS;
    return MOCK_MARKETS.filter((m) => m.symbol.toLowerCase().includes(s));
  }, [q]);

  return (
    <header className="sticky top-0 z-20 border-b border-white/10 bg-black/80 backdrop-blur">
    <div className="mx-auto flex h-16 max-w-7xl items-center gap-3 px-4">
      {/* Left: Logo + name */}
      <Link href="/" className="flex items-center gap-2">
      <div className="flex items-center gap-3 font-semibold">
            <Gi3dGlasses size={24}/>
            <span className="text-xl font-bold">Trueman</span>
          </div>
      </Link>

      {/* Middle: Search + Spot pill */}
      <div className="mx-3 flex flex-1 items-center gap-3">
        <div className="hidden items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-2 md:flex md:min-w-[340px]">
          <IoSearch />
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="Search markets (e.g. BTC, SOL)"
            className="w-full bg-transparent text-sm outline-none placeholder:text-zinc-400"
          />
        </div>
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

function Logo() {
    return (
      <svg width="26" height="26" viewBox="0 0 24 24" aria-hidden className="block">
        <defs>
          <linearGradient id="gx" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#7C3AED" />
            <stop offset="100%" stopColor="#06B6D4" />
          </linearGradient>
        </defs>
        <path fill="url(#gx)" d="M12 2l9 5v10l-9 5-9-5V7l9-5zm0 2.2L5 7v8l7 3.8L19 15V7l-7-2.8z" />
      </svg>
    );
  }