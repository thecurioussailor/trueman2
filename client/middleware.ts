import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

export function middleware(req: NextRequest) {
    const { pathname, search } = req.nextUrl;

    // Public routes (not protected)
    if (pathname.startsWith("/login") || pathname.startsWith("/signup")) {
        return NextResponse.next();
    }

    // Read JWT from cookie
    const token = req.cookies.get("authToken")?.value;

    // If missing, redirect to login with return URL
    if (!token) {
        const url = req.nextUrl.clone();
        url.pathname = "/login";
        url.search = `?next=${encodeURIComponent(pathname + search)}`;
        return NextResponse.redirect(url);
    }

return NextResponse.next();
}

export const config = {
    matcher: [
        "/exchange",
        "/trade/:path*",
        "/admin/:path*",
    ],
};