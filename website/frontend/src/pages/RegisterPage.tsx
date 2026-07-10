import { useState, FormEvent } from "react";
import { Link, Navigate, useNavigate } from "react-router-dom";
import { useAuthStore } from "@/stores/auth";
import { UserPlus } from "lucide-react";

export default function RegisterPage() {
  const navigate = useNavigate();
  const register = useAuthStore((s) => s.register);
  const user = useAuthStore((s) => s.user);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  if (user) return <Navigate to="/" replace />;

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError("");

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }
    if (password.length < 8) {
      setError("Password must be at least 8 characters");
      return;
    }

    setLoading(true);
    try {
      await register(email, password, displayName || undefined);
      navigate("/");
    } catch (err: any) {
      setError(err.message || "Registration failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-screen flex items-center justify-center bg-zinc-950">
      <div className="w-full max-w-sm mx-auto px-6">
        <div className="text-center mb-8">
          <h1 className="text-2xl font-semibold text-zinc-100">Create Account</h1>
          <p className="text-sm text-zinc-500 mt-1">Join Prism clip sharing</p>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {error && (
            <div className="px-4 py-2.5 rounded-lg bg-red-950/60 border border-red-900/60">
              <p className="text-xs text-red-300">{error}</p>
            </div>
          )}

          <div>
            <label className="block text-xs text-zinc-500 mb-1.5">Display name (optional)</label>
            <input
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              placeholder="Your name"
            />
          </div>

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
              placeholder="At least 8 characters"
            />
          </div>

          <div>
            <label className="block text-xs text-zinc-500 mb-1.5">Confirm password</label>
            <input
              type="password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              className="w-full bg-zinc-900 border border-zinc-800 rounded-lg px-3 py-2 text-sm text-zinc-100 placeholder-zinc-600 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              placeholder="Repeat your password"
            />
          </div>

          <button
            type="submit"
            disabled={loading}
            className="w-full flex items-center justify-center gap-2 bg-zinc-100 text-zinc-950 rounded-lg px-4 py-2 text-sm font-medium hover:bg-zinc-200 transition-colors disabled:opacity-50"
          >
            <UserPlus className="size-4" />
            {loading ? "Creating account..." : "Create account"}
          </button>

          <p className="text-center text-xs text-zinc-600">
            Already have an account?{" "}
            <Link to="/login" className="text-zinc-400 hover:text-zinc-200">
              Sign in
            </Link>
          </p>
        </form>
      </div>
    </div>
  );
}
