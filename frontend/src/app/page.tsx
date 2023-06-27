'use client';
import { useUser } from '@auth0/nextjs-auth0/client';

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
        <a href="/api/auth/logout">Logout</a>

      </div>
    );
  }

  return <a href="/api/auth/login">Login</a>;
}