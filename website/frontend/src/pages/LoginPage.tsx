import { useState, FormEvent } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/auth";
import { LogIn } from "lucide-react";

export default function LoginPage() {
  const navigate = useNavigate();
  const login = useAuthStore((s) => s.login);
  const user = useAuthStore((s) => s.user);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  if (user) return <Navigate to="/" replace />;

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      await login(email, password);
      navigate("/");
    } catch (err: any) {
      setError(err.message || "Login failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-screen flex items-center justify-center bg-zinc-950">
      <div className="w-full max-w-sm mx-auto px-6">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-semibold text-zinc-100">Prism</h1>
          <p className="text-sm text-zinc-500 mt-1">Sign in to your account</p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="px-4 py-2.5 rounded-lg bg-red-950/60 border border-red-900/60">
              <p className="text-xs text-red-300">{error}</p>
            </div>
          )}

          <div>
            <label className="block text-xs text-zinc-500 mb-1.5">Email</label>
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              placeholder="you@example.com"
            />
          </div>

          <div>
            <label className="block text-xs text-zinc-500 mb-1.5">Password</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              placeholder="Enter your password"
            />
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full flex items-center justify-center gap-2 bg-zinc-100 text-zinc-950 rounded-lg px-4 py-2 text-sm font-medium hover:bg-zinc-200 transition-colors disabled:opacity-50"
          >
            <LogIn className="size-4" />
            {loading ? "Signing in..." : "Sign in"}
          </button>

          <p className="text-center text-xs text-zinc-600">
            Don't have an account?{" "}
            <Link to="/register" className="text-zinc-400 hover:text-zinc-200">
              Register
            </Link>
          </p>
        </form>
      </div>
    </div>
  );
}
