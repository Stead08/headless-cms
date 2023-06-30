'use client';
import { useUser } from '@auth0/nextjs-auth0/client';
import Link from "next/link";

export default function Home() {
  const { user, error, isLoading } = useUser();

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>{error.message}</div>;

  if (user) {
    return (
      <div>
        <h1>Welcome {user.name}! </h1>
        <p>{user.email}</p>
        <p>{user.nickname}</p>
        <Link href="/dashboard">Dashboard</Link>
        <Link href="/api/auth/logout">Logout</Link>

      </div>
    );
  }

  return <a href="/api/auth/login">Login</a>;
}