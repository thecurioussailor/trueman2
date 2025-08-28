import axios from "axios";

export const API_BASE = process.env.NEXT_PUBLIC_API_BASE_URL || "http://127.0.0.1:8080";

const getCookie = (name: string) => {
  if (typeof document === "undefined") return null;
  const m = document.cookie.match(new RegExp(`(^| )${name}=([^;]+)`));
  return m ? decodeURIComponent(m[1]) : null;
}

export const api = axios.create({
    baseURL: API_BASE,
    headers: {
        "Content-Type": "application/json",
    },
});

api.interceptors.request.use((config) => {
    if (typeof window !== "undefined") {
        const isToken = localStorage.getItem("authToken");
        const cookieToken = getCookie("authToken");
        const token = isToken || cookieToken;
        if (token) config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
});

// unwrap data and normalize errors
api.interceptors.response.use(
    (res) => res,
    (err) => {
      const status = err?.response?.status;
      if (status === 401 && typeof window !== "undefined") {
        try {
          localStorage.removeItem("authToken");
          document.cookie = "authToken=; Max-Age=0; Path=/; SameSite=Lax";
        } catch (e) {
          const next = encodeURIComponent(window.location.pathname + window.location.search);
          window.location.href = `/login?next=${next}`;
        }
      }
      const msg =
        err?.response?.data?.message ||
        (typeof err?.response?.data === "string" ? err.response.data : "") ||
        err.message ||
        "Request failed";
      return Promise.reject(new Error(msg));
    }
  );