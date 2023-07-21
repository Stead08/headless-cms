import { useAuth0 } from '@auth0/auth0-react';

const Settings = () => {
  const { user, isAuthenticated, isLoading } = useAuth0();
  if (!isAuthenticated) {
    return <h1>Please Login</h1>;
  }

  if (isLoading) {
    return <div>Loading ...</div>;
  }

  return (
    <>
      <h1>Hello, {user?.name}</h1>

      <img src={user?.picture} alt={user?.name} />
    </>
  );
};

export default Settings;
