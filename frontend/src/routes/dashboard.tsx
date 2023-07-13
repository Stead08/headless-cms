import { useAuth0 } from '@auth0/auth0-react';
import { Link } from 'react-router-dom';
export const Dashboard = () => {
  const { user, isAuthenticated, isLoading } = useAuth0();

  if (!user) {
    return (
      <div>
        <h2>Please Login</h2>
        <Link to="/">Home</Link>
      </div>
    );
  }
  if (isLoading) {
    return <div>Loading ...</div>;
  }

  return (
    isAuthenticated ?? (
      <>
        <img src={user?.picture} alt={user?.name} />
        <h2>{user?.name}</h2>
        <p>{user?.email}</p>
      </>
    )
  );
};

export default Dashboard;
